# Pydantic Logfire — Observability for Agentic AI Systems

Pydantic Logfire is a production-grade observability platform built on OpenTelemetry, designed to give full visibility into AI agent behavior, LLM interactions, tool calls, API requests, database queries, and everything in between. It is built by the team behind Pydantic and integrates natively with Pydantic AI.

The SDK is open source (MIT licensed). The platform (UI and backend) is closed source with a generous free tier of 10 million spans/metrics per month.

```bash
pip install pydantic-ai           # includes logfire SDK
# or standalone:
pip install logfire
```

## Core Concepts

Logfire's observability model is built on four OpenTelemetry primitives:

**Span** is the atomic unit of telemetry — a single step in an operation that records what happened, how long it took, and any associated metadata. Think of spans as structured logs with duration and parent-child relationships.

**Trace** is a tree of spans representing the full lifecycle of an operation. For an agentic system, a single trace might capture the user request, agent reasoning loop, multiple LLM calls, tool invocations, Redis lookups, and the final response — all linked together in a hierarchical timeline.

**Metric** is a numeric value aggregated over time — request latency, token counts, CPU load, queue depth. Metrics power dashboards, SLOs, and alerts.

**Log** is a timestamped event without duration. Logs capture discrete events within traces.

## Getting Started

### Authentication and Project Setup

```bash
# Authenticate with Logfire
logfire auth

# Create a project
logfire projects new
```

This writes a `.logfire` directory with your write token. The SDK reads it automatically at runtime.

### Minimal Instrumentation

```python
import logfire
from pydantic_ai import Agent

logfire.configure()
logfire.instrument_pydantic_ai()

agent = Agent(
    'openai:gpt-4o',
    instructions='Be concise, reply with one sentence.',
)

result = agent.run_sync('Where does "hello world" come from?')
print(result.output)
```

With these two lines — `logfire.configure()` and `logfire.instrument_pydantic_ai()` — every agent run automatically generates a trace with spans for each model request and tool call. No other code changes required.

## Instrumenting an Agentic System

A real-world agent interacts with LLMs, external APIs, databases, and other services. Logfire provides one-line instrumentation for all of these, producing unified traces that show the full request lifecycle.

### Multi-Layer Instrumentation

```python
import logfire
from pydantic_ai import Agent

logfire.configure()

# Instrument everything your agent touches
logfire.instrument_pydantic_ai()       # Agent runs, LLM calls, tool execution
logfire.instrument_httpx()             # Outbound HTTP requests (API calls)
logfire.instrument_redis()             # Redis operations (agent coordination)
logfire.instrument_fastapi(app)        # Inbound web requests
```

A single user request now produces a trace spanning the FastAPI handler → agent run → LLM call → tool execution → Redis lookup → HTTP API call, all nested in a hierarchical tree with timing information at every node.

### Agent with Tools — Full Example

```python
from __future__ import annotations
from pydantic_ai import Agent, RunContext
import logfire
import httpx

logfire.configure()
logfire.instrument_pydantic_ai()
logfire.instrument_httpx()

market_agent = Agent(
    'openai:gpt-4o',
    deps_type=httpx.AsyncClient,
    system_prompt='You are a market data analyst. Use available tools to fetch data.',
)

@market_agent.tool
async def fetch_price(ctx: RunContext[httpx.AsyncClient], symbol: str) -> str:
    """Fetch the current price for a given ticker symbol."""
    response = await ctx.deps.get(
        f'https://api.example.com/price/{symbol}'
    )
    data = response.json()
    return f"{symbol}: ${data['price']}"

@market_agent.tool
async def fetch_fundamentals(ctx: RunContext[httpx.AsyncClient], symbol: str) -> str:
    """Fetch fundamental data (P/E, market cap) for a ticker."""
    response = await ctx.deps.get(
        f'https://api.example.com/fundamentals/{symbol}'
    )
    return response.text

async def main():
    async with httpx.AsyncClient() as client:
        result = await market_agent.run(
            'Compare the P/E ratios of AAPL and MSFT',
            deps=client,
        )
        print(result.output)
```

In Logfire, this produces a trace like:

```
▸ agent run (market_agent)                           2.3s
  ├─ model request (gpt-4o)                          0.8s  [input: 145 tokens, output: 62 tokens]
  ├─ running tool: fetch_fundamentals("AAPL")        0.3s
  │  └─ GET https://api.example.com/fundamentals/AAPL  0.3s
  ├─ running tool: fetch_fundamentals("MSFT")        0.2s
  │  └─ GET https://api.example.com/fundamentals/MSFT  0.2s
  └─ model request (gpt-4o)                          1.0s  [input: 380 tokens, output: 95 tokens]
```

Every span records duration, inputs, outputs, token counts, and exceptions — all queryable via SQL.

## Manual Spans and Logs

Beyond auto-instrumentation, you can add custom spans to trace domain-specific operations:

### Spans (Operations with Duration)

```python
import logfire

logfire.configure()

with logfire.span('portfolio_rebalance', strategy='momentum', num_assets=25):
    # ... rebalancing logic ...
    
    with logfire.span('risk_assessment'):
        var_95 = calculate_var(portfolio, confidence=0.95)
        logfire.info('VaR calculated', var_95=var_95)
    
    with logfire.span('order_generation'):
        orders = generate_orders(portfolio, target_weights)
        logfire.info('Orders generated', count=len(orders))
```

Child spans nest automatically within parent spans, producing a clean hierarchy in the Logfire UI.

### The `@logfire.instrument` Decorator

For functions you want to trace without context managers:

```python
import logfire

logfire.configure()

@logfire.instrument('Fetching holdings for {portfolio_id}')
def fetch_holdings(portfolio_id: str) -> list[dict]:
    # ... database query ...
    return holdings

@logfire.instrument('Computing risk metrics')
async def compute_risk(positions: list[dict]) -> dict:
    # ... risk computation ...
    return metrics
```

The decorator creates a span around every call, automatically extracting function arguments as span attributes.

### Log Levels

```python
logfire.debug('Cache hit for {symbol}', symbol='AAPL')
logfire.info('Agent completed analysis', tokens_used=450)
logfire.warn('Rate limit approaching', remaining=5)
logfire.error('API call failed', status_code=429, endpoint='/v1/prices')
```

If a span finishes with an unhandled exception, its level is automatically set to `error` and the traceback is captured.

## LLM Cost and Token Tracking

Logfire automatically captures token usage and cost data from Pydantic AI agent runs following OpenTelemetry GenAI semantic conventions. Each LLM span records:

- `gen_ai.usage.input_tokens` — prompt tokens consumed
- `gen_ai.usage.output_tokens` — completion tokens generated
- `gen_ai.request.model` — the model used
- `operation.cost` — estimated cost in USD

### Aggregating Across Multi-Agent Runs

For agentic systems with multiple sequential or parallel LLM calls, you can aggregate token usage on a parent span:

```python
from pydantic_ai import Agent
import logfire

logfire.configure(
    metrics=logfire.MetricsOptions(collect_in_spans=True),
)
logfire.instrument_pydantic_ai()

analyst = Agent('openai:gpt-4o', system_prompt='You are a financial analyst.')
risk_agent = Agent('openai:gpt-4o', system_prompt='You assess portfolio risk.')

with logfire.span('full_analysis_pipeline'):
    analysis = analyst.run_sync('Analyze AAPL earnings trends')
    risk = risk_agent.run_sync(
        f'Assess risk for this analysis: {analysis.output}'
    )
```

The outer `full_analysis_pipeline` span now contains aggregated `gen_ai.client.token.usage` and `operation.cost` metrics across both agent runs — no manual counting needed.

### Token Usage in the UI

In the Logfire live view, LLM spans display a token usage badge (coin icon). Parent spans show a ∑ symbol indicating the sum across all child LLM calls. The built-in "LLM Tokens and Costs" dashboard breaks down input/output tokens by model over time.

## Querying Data with SQL

One of Logfire's most powerful features is direct SQL access to all your telemetry data. Every span, log, and metric is stored in a queryable `records` table.

### Example: Agent Performance Summary

```sql
SELECT
    span_name,
    COUNT(*) as runs,
    AVG(duration) as avg_duration_ms,
    AVG(CAST(attributes->>'gen_ai.usage.input_tokens' AS INT)) as avg_input_tokens,
    AVG(CAST(attributes->>'gen_ai.usage.output_tokens' AS INT)) as avg_output_tokens
FROM records
WHERE otel_scope_name = 'pydantic-ai'
  AND span_name = 'agent run'
  AND start_timestamp > NOW() - INTERVAL '24 hours'
GROUP BY span_name
```

### Example: Slowest Tool Calls

```sql
SELECT
    message,
    duration,
    start_timestamp,
    attributes->>'tool_arguments' as args
FROM records
WHERE span_name LIKE 'running tool%'
ORDER BY duration DESC
LIMIT 20
```

### Example: Error Rate by Agent

```sql
SELECT
    attributes->>'gen_ai.request.model' as model,
    COUNT(*) as total,
    SUM(CASE WHEN is_exception THEN 1 ELSE 0 END) as errors,
    ROUND(100.0 * SUM(CASE WHEN is_exception THEN 1 ELSE 0 END) / COUNT(*), 2) as error_pct
FROM records
WHERE otel_scope_name = 'pydantic-ai'
  AND start_timestamp > NOW() - INTERVAL '7 days'
GROUP BY model
ORDER BY error_pct DESC
```

The Logfire UI also includes a natural-language-to-SQL feature (powered by Pydantic AI) so you can query your data conversationally.

## Custom Metrics

Beyond auto-captured LLM metrics, you can define your own:

```python
import logfire

logfire.configure()

# Create metrics once at module level
agent_latency = logfire.metric_histogram(
    'agent.response_latency',
    unit='ms',
    description='End-to-end agent response time',
)

tool_call_counter = logfire.metric_counter(
    'agent.tool_calls',
    description='Number of tool invocations by the agent',
)

# Record values at runtime
def handle_agent_request(query: str):
    import time
    start = time.perf_counter()
    
    result = agent.run_sync(query)
    
    latency = (time.perf_counter() - start) * 1000
    agent_latency.record(latency)
    tool_call_counter.add(len(result.all_messages()))
    
    return result.output
```

These metrics flow into Logfire dashboards where you can chart p50/p95 latencies, set SLO thresholds, and configure alerts.

## Distributed Tracing

When your agentic system spans multiple services — a FastAPI gateway, agent workers, a separate risk engine — Logfire propagates trace context automatically across HTTP boundaries.

### Automatic Propagation

Instrumented web frameworks (FastAPI, Flask, Django) and HTTP clients (httpx, requests) automatically inject and extract the `traceparent` header, linking spans across services into a single trace.

```python
# Service A: API Gateway
import logfire
from fastapi import FastAPI

app = FastAPI()
logfire.configure(service_name='api-gateway')
logfire.instrument_fastapi(app)
logfire.instrument_httpx()

@app.post('/analyze')
async def analyze(query: str):
    async with httpx.AsyncClient() as client:
        # traceparent header is automatically injected
        response = await client.post(
            'http://agent-service:8001/run',
            json={'query': query},
        )
    return response.json()
```

```python
# Service B: Agent Worker
import logfire
from fastapi import FastAPI
from pydantic_ai import Agent

app = FastAPI()
logfire.configure(service_name='agent-worker')
logfire.instrument_fastapi(app)  # automatically extracts traceparent
logfire.instrument_pydantic_ai()

agent = Agent('openai:gpt-4o')

@app.post('/run')
async def run_agent(query: str):
    result = await agent.run(query)
    return {'output': result.output}
```

The resulting trace in Logfire shows the full journey: API Gateway → Agent Worker → LLM call, all in one view.

### Manual Context Propagation

For non-HTTP communication (Redis pub/sub, message queues):

```python
import logfire

# Producer
with logfire.span('dispatch_task'):
    ctx = logfire.get_context()
    # serialize ctx and send via Redis/Kafka/etc.

# Consumer
with logfire.attach_context(ctx):
    logfire.info('processing task')
    # this span is linked to the producer's trace
```

### Thread and Process Pool Propagation

Logfire automatically patches `ThreadPoolExecutor` and `ProcessPoolExecutor` so spans in child threads/processes are correctly linked to their parent:

```python
from concurrent.futures import ThreadPoolExecutor
import logfire

logfire.configure()

@logfire.instrument('Analyzing {symbol}')
def analyze_symbol(symbol: str) -> dict:
    # ... analysis logic ...
    return {'symbol': symbol, 'score': 0.85}

with logfire.span('batch_analysis'):
    executor = ThreadPoolExecutor(max_workers=4)
    results = list(executor.map(analyze_symbol, ['AAPL', 'MSFT', 'GOOG', 'AMZN']))
```

All four `analyze_symbol` spans appear as children of `batch_analysis` in the trace.

## Integration Ecosystem

Logfire provides one-line auto-instrumentation for a broad ecosystem. Each integration uses a `logfire.instrument_<package>()` call:

**LLM Clients & AI Frameworks:**
`instrument_pydantic_ai()`, `instrument_openai()`, `instrument_anthropic()`, plus LangChain, LlamaIndex, Mirascope, LiteLLM, Magentic

**Web Frameworks:**
`instrument_fastapi()`, `instrument_django()`, `instrument_flask()`, `instrument_starlette()`, `instrument_aiohttp()`

**Database Clients:**
`instrument_psycopg()`, `instrument_sqlalchemy()`, `instrument_asyncpg()`, `instrument_pymongo()`, `instrument_redis()`, `instrument_sqlite3()`

**HTTP Clients:**
`instrument_httpx()`, `instrument_requests()`

**Task Queues:**
`instrument_celery()`, plus Airflow, FastStream

**Other:**
`instrument_mcp()` (MCP protocol), `instrument_pydantic()` (validation tracing), `instrument_system_metrics()`, Stripe, AWS Lambda

Since Logfire is built on OpenTelemetry, any OTel-compatible instrumentation works automatically — these are convenience wrappers, not requirements.

### Instrumenting HTTPX with Full Capture

For debugging agent tool calls that hit external APIs:

```python
import logfire

logfire.configure()
logfire.instrument_httpx(capture_all=True)
# captures request headers, request body, response headers, response body
```

### Instrumenting Redis

Relevant for agent coordination via Redis pub/sub or caching:

```python
import logfire
import redis

logfire.configure()
logfire.instrument_redis(capture_statement=True)

client = redis.StrictRedis(host='localhost', port=6379)
client.set('agent:market:last_signal', 'bullish')
```

Each Redis command generates a span with timing and (optionally) the command text.

## Dashboards and Alerts

### Standard Dashboards

Logfire ships with pre-built dashboards you can enable per project:

- **LLM Tokens and Costs** — Input/output token usage by model over time, with cost breakdown
- **Basic System Metrics** — CPU, memory, swap, process count

### Custom Dashboards

Build your own using SQL queries. Each chart is backed by a time-series or aggregation query:

```sql
-- Chart: Agent runs per hour (time-series)
SELECT
    time_bucket('1 hour', start_timestamp) as ts,
    COUNT(*) as runs
FROM records
WHERE span_name = 'agent run'
GROUP BY ts
ORDER BY ts
```

```sql
-- Chart: Tool call distribution (bar chart)
SELECT
    message as tool_name,
    COUNT(*) as invocations
FROM records
WHERE span_name LIKE 'running tool%'
  AND start_timestamp > NOW() - INTERVAL '24 hours'
GROUP BY message
ORDER BY invocations DESC
```

You can define dashboard variables for dynamic filtering (e.g., by model, agent name, or environment).

## Sending Data to Alternative Backends

Because Logfire is built on OpenTelemetry, you can send data to any OTel-compatible backend — or to both Logfire and your own infrastructure simultaneously.

### Using a Custom OTel Backend

```python
import os
import logfire
from pydantic_ai import Agent

os.environ['OTEL_EXPORTER_OTLP_ENDPOINT'] = 'http://localhost:4318'

logfire.configure(send_to_logfire=False)  # only send to custom backend
logfire.instrument_pydantic_ai()

agent = Agent('openai:gpt-4o')
result = agent.run_sync('What is the capital of France?')
```

### Without Logfire SDK at All

```python
from opentelemetry.exporter.otlp.proto.http.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.trace import set_tracer_provider
from pydantic_ai import Agent

exporter = OTLPSpanExporter()
tracer_provider = TracerProvider()
tracer_provider.add_span_processor(BatchSpanProcessor(exporter))
set_tracer_provider(tracer_provider)

Agent.instrument_all()

agent = Agent('openai:gpt-4o')
result = agent.run_sync('What is the capital of France?')
```

## Connecting Logfire to Pydantic Evals

Logfire and Pydantic Evals form a feedback loop: Evals test your agents systematically, and Logfire captures the traces for every eval run, giving you full debugging context when an evaluation fails.

```python
from pydantic_ai import Agent
from pydantic_evals import Case, Dataset
from pydantic_evals.evaluators import LLMJudge, IsInstance
import logfire

logfire.configure(
    send_to_logfire='if-token-present',
    environment='evaluation',
    service_name='agent-evals',
)
logfire.instrument_pydantic_ai()

agent = Agent('openai:gpt-4o', system_prompt='You are a financial analyst.')

async def analyst_task(question: str) -> str:
    result = await agent.run(question)
    return result.output

dataset = Dataset(
    cases=[
        Case(
            name='pe_ratio_explanation',
            inputs='What does P/E ratio mean?',
            expected_output='Price-to-earnings ratio',
            evaluators=[
                LLMJudge(
                    rubric='Accurate, concise, suitable for finance professional',
                    include_input=True,
                ),
            ],
        ),
    ],
    evaluators=[IsInstance(type_name='str')],
)

report = dataset.evaluate_sync(analyst_task)
report.print()
# Full traces for each eval case are now visible in Logfire
```

In the Logfire UI, you can inspect the complete trace for each evaluation case — the agent's reasoning, every LLM call, tool invocations, token usage — making it straightforward to debug why a particular case scored poorly.

## Querying Logfire Data via API

For building custom analytics pipelines or exporting data to your own warehouse:

```python
import requests

READ_TOKEN = 'your-read-token'
BASE_URL = 'https://logfire-us.pydantic.dev'

query = """
SELECT
    trace_id,
    span_name,
    duration,
    attributes->>'gen_ai.request.model' as model,
    attributes->>'gen_ai.usage.input_tokens' as input_tokens,
    attributes->>'gen_ai.usage.output_tokens' as output_tokens
FROM records
WHERE otel_scope_name = 'pydantic-ai'
ORDER BY start_timestamp DESC
LIMIT 100
"""

response = requests.get(
    f'{BASE_URL}/v1/query',
    params={'sql': query},
    headers={'Authorization': f'Bearer {READ_TOKEN}'},
)

data = response.json()
```

This lets you pipe agent telemetry into S3, a data lake, or a BI tool for long-term retention and custom reporting beyond Logfire's 30-day default window.

## Sampling and Cost Control

For high-throughput agentic systems, you may want to limit the volume of telemetry data:

```python
from opentelemetry.sdk.trace.sampling import TraceIdRatioBased

logfire.configure(
    sampling=TraceIdRatioBased(0.1),  # sample 10% of traces
)
```

Logfire pricing is based on spans/metrics shipped. The free tier covers 10 million units/month. Beyond that, it's $2 per million. There are no charges for seats, hosts, or projects.

## Key Takeaways for Agentic Systems

For a multi-agent system like a crypto trading network with specialized agents for market analysis, risk management, and tax accounting coordinated via Redis:

1. **Instrument everything in ~5 lines**: `instrument_pydantic_ai()` + `instrument_redis()` + `instrument_httpx()` gives you full visibility across agents, coordination layer, and external APIs.

2. **Token costs aggregate automatically**: Parent spans collect total token usage and cost across all child agent runs — critical for budgeting per-trade or per-analysis-cycle costs.

3. **SQL-queryable traces**: Build dashboards tracking agent decision quality, tool call latency, error rates, and cost trends over time.

4. **Distributed traces link agents**: When agents communicate via Redis pub/sub or HTTP, trace context propagation shows the full multi-agent workflow as a single trace tree.

5. **Evals + Logfire feedback loop**: Run Pydantic Evals to test agent accuracy, then drill into Logfire traces to debug failures — closing the loop between evaluation and improvement.
