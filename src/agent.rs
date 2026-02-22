//! Agent subprocess manager.
//!
//! Spawns Python agent processes and bridges their stdin/stdout/stderr
//! with the TUI via JSON-lines over pipes.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use crate::models::balance::BalanceData;
use crate::models::execution::ExecutionData;
use crate::models::ticker::TickerData;
use crate::models::trade::TradeData;
use crate::tui::Message;

/// Commands sent from the TUI to an agent subprocess.
#[derive(Debug, Clone)]
pub enum AgentCommand {
    /// A user-typed message to forward to the agent.
    UserMessage(String),
    /// Risk limits description for the agent's system prompt.
    RiskLimits(String),
    /// Structured result of an order placement attempt.
    OrderResponse {
        success: bool,
        order_id: Option<String>,
        cl_ord_id: Option<String>,
        order_userref: Option<i64>,
        error: Option<String>,
    },
    /// Token state changed — agents should know if orders can be submitted.
    TokenState(String),
    /// Order status changes and trade execution events.
    ExecutionUpdate(Vec<ExecutionData>),
    /// Throttled price snapshot for a single trading pair.
    TickerUpdate(TickerData),
    /// Market trades.
    TradeUpdate(Vec<TradeData>),
    /// Balance changes.
    BalanceUpdate(Vec<BalanceData>),
    /// Active trading pairs selected by the operator.
    ActivePairs(Vec<String>),
    /// Request the agent to shut down gracefully.
    Shutdown,
}

/// Handle to a running agent subprocess.
///
/// Dropping the handle kills the child process (via `kill_on_drop`).
pub struct AgentHandle {
    /// Sender for commands to the agent's stdin writer task.
    pub commands: mpsc::UnboundedSender<AgentCommand>,
    /// The child process (kept alive; killed on drop).
    _child: Child,
}

/// JSON message from a Python agent (stdout).
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AgentToTui {
    Output {
        agent: usize,
        line: String,
    },
    Ready,
    Error {
        message: String,
    },
    PlaceOrder {
        symbol: String,
        side: String,
        order_type: String,
        qty: String,
        #[serde(default)]
        price: Option<String>,
        #[serde(default)]
        cl_ord_id: Option<String>,
    },
}

/// JSON message from the TUI to a Python agent (stdin).
#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TuiToAgent {
    UserMessage {
        content: String,
    },
    RiskLimits {
        description: String,
    },
    OrderResponse {
        success: bool,
        order_id: Option<String>,
        cl_ord_id: Option<String>,
        order_userref: Option<i64>,
        error: Option<String>,
    },
    TokenState {
        state: String,
    },
    ExecutionUpdate {
        data: Vec<ExecutionData>,
    },
    TickerUpdate {
        data: TickerData,
    },
    TradeUpdate {
        data: Vec<TradeData>,
    },
    BalanceUpdate {
        data: Vec<BalanceData>,
    },
    ActivePairs {
        pairs: Vec<String>,
    },
    Shutdown,
}

/// Spawns a Python agent subprocess and wires its I/O to the TUI message channel.
///
/// Returns an [`AgentHandle`] that can be used to send commands to the agent.
/// The child process is killed automatically when the handle is dropped.
pub fn spawn_agent(
    agent_index: usize,
    script_path: &str,
    tx: mpsc::Sender<Message>,
) -> crate::Result<AgentHandle> {
    let child = Command::new("python3")
        .arg(script_path)
        .arg(agent_index.to_string())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| crate::LeesonError::Io(format!("failed to spawn agent {agent_index}: {e}")))?;

    wire_agent_io(agent_index, child, tx)
}

/// Spawns the multi-agent Python module via `uv run` in the agents directory.
///
/// Uses `uv run python -m multi_agent` so that the uv-managed virtual
/// environment and all dependencies are available.
pub fn spawn_multi_agent(
    agent_index: usize,
    tx: mpsc::Sender<Message>,
) -> crate::Result<AgentHandle> {
    let child = Command::new("uv")
        .args([
            "run",
            "--directory",
            "agents",
            "python",
            "-m",
            "multi_agent",
        ])
        .arg(agent_index.to_string())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| crate::LeesonError::Io(format!("failed to spawn multi-agent system: {e}")))?;

    wire_agent_io(agent_index, child, tx)
}

/// Wires a child process's stdin/stdout/stderr to the TUI message channel.
fn wire_agent_io(
    agent_index: usize,
    mut child: Child,
    tx: mpsc::Sender<Message>,
) -> crate::Result<AgentHandle> {
    let stdout = child
        .stdout
        .take()
        .expect("stdout piped but missing from child");
    let stderr = child
        .stderr
        .take()
        .expect("stderr piped but missing from child");
    let stdin = child
        .stdin
        .take()
        .expect("stdin piped but missing from child");

    // Channel for TUI → agent commands
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<AgentCommand>();

    // Stdout reader: parse JSON-lines, forward to TUI
    spawn_stdout_reader(agent_index, stdout, tx.clone());

    // Stderr reader: forward lines as "[stderr] ..." to agent panel
    spawn_stderr_reader(agent_index, stderr, tx.clone());

    // Stdin writer: serialize commands to JSON-lines
    spawn_stdin_writer(agent_index, stdin, cmd_rx, tx);

    Ok(AgentHandle {
        commands: cmd_tx,
        _child: child,
    })
}

/// Reads stdout from the agent, parses JSON-lines, and sends messages to the TUI.
fn spawn_stdout_reader(
    agent_index: usize,
    stdout: tokio::process::ChildStdout,
    tx: mpsc::Sender<Message>,
) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            match serde_json::from_str::<AgentToTui>(&line) {
                Ok(AgentToTui::Output { agent, line }) => {
                    let _ = tx.try_send(Message::AgentOutput {
                        agent_index: agent,
                        line,
                    });
                }
                Ok(AgentToTui::Ready) => {
                    let _ = tx.try_send(Message::AgentReady(agent_index));
                }
                Ok(AgentToTui::Error { message }) => {
                    let _ = tx.try_send(Message::AgentOutput {
                        agent_index,
                        line: format!("[error] {message}"),
                    });
                }
                Ok(AgentToTui::PlaceOrder {
                    symbol,
                    side,
                    order_type,
                    qty,
                    price,
                    cl_ord_id,
                }) => {
                    let _ = tx.try_send(Message::AgentOrderRequest {
                        agent_index,
                        symbol,
                        side,
                        order_type,
                        qty,
                        price,
                        cl_ord_id,
                    });
                }
                Err(_) => {
                    // Non-JSON line — pass through as raw output
                    let _ = tx.try_send(Message::AgentOutput { agent_index, line });
                }
            }
        }
        let _ = tx.try_send(Message::AgentExited {
            agent_index,
            error: None,
        });
    });
}

/// Reads stderr from the agent and forwards lines to the agent's TUI panel.
fn spawn_stderr_reader(
    agent_index: usize,
    stderr: tokio::process::ChildStderr,
    tx: mpsc::Sender<Message>,
) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx.try_send(Message::AgentOutput {
                agent_index,
                line: format!("[stderr] {line}"),
            });
        }
    });
}

/// Reads commands from the TUI channel, serializes to JSON-lines, writes to agent stdin.
fn spawn_stdin_writer(
    agent_index: usize,
    mut stdin: tokio::process::ChildStdin,
    mut cmd_rx: mpsc::UnboundedReceiver<AgentCommand>,
    tx: mpsc::Sender<Message>,
) {
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            let msg = match cmd {
                AgentCommand::UserMessage(content) => TuiToAgent::UserMessage { content },
                AgentCommand::RiskLimits(description) => TuiToAgent::RiskLimits { description },
                AgentCommand::OrderResponse {
                    success,
                    order_id,
                    cl_ord_id,
                    order_userref,
                    error,
                } => TuiToAgent::OrderResponse {
                    success,
                    order_id,
                    cl_ord_id,
                    order_userref,
                    error,
                },
                AgentCommand::TokenState(state) => TuiToAgent::TokenState { state },
                AgentCommand::ExecutionUpdate(data) => TuiToAgent::ExecutionUpdate { data },
                AgentCommand::TickerUpdate(data) => TuiToAgent::TickerUpdate { data },
                AgentCommand::TradeUpdate(data) => TuiToAgent::TradeUpdate { data },
                AgentCommand::BalanceUpdate(data) => TuiToAgent::BalanceUpdate { data },
                AgentCommand::ActivePairs(pairs) => TuiToAgent::ActivePairs { pairs },
                AgentCommand::Shutdown => TuiToAgent::Shutdown,
            };
            let mut json =
                serde_json::to_string(&msg).expect("TuiToAgent serialization should not fail");
            json.push('\n');
            if let Err(e) = stdin.write_all(json.as_bytes()).await {
                let _ = tx.try_send(Message::AgentExited {
                    agent_index,
                    error: Some(format!("stdin write failed: {e}")),
                });
                break;
            }
            if stdin.flush().await.is_err() {
                break;
            }
        }
    });
}
