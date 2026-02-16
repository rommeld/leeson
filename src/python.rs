//! PyO3 Python module definition and Python-facing order parameter types.
//!
//! The four order parameter types (`AddOrder`, `CancelOrder`, `AmendOrder`,
//! `EditOrder`) mirror the internal Rust params but omit the auth token.
//! Python agents construct order intents; the Rust integration layer adds
//! the token when executing.

use pyo3::prelude::*;
use rust_decimal::Decimal;

use crate::models::Channel;
use crate::models::add_order::{
    AddOrderResponse, AddOrderResult, ConditionalOrder, FeeCurrencyPreference, OrderSide,
    OrderType, StpType, TimeInForce, TriggerParams, TriggerPriceType, TriggerReference,
};
use crate::models::amend_order::{AmendOrderResponse, AmendOrderResult, PriceType};
use crate::models::balance::{BalanceData, BalanceUpdateData, WalletBalance};
use crate::models::batch_add::{BatchAddOrderResult, BatchAddResponse};
use crate::models::batch_cancel::{BatchCancelResponse, BatchCancelResult};
use crate::models::book::{BookData, BookDepth, PriceLevel};
use crate::models::cancel_after::{CancelAfterResponse, CancelAfterResult};
use crate::models::cancel_all::{CancelAllResponse, CancelAllResult};
use crate::models::cancel_order::{CancelOrderResponse, CancelOrderResult};
use crate::models::candle::CandleData;
use crate::models::edit_order::{EditOrderResponse, EditOrderResult};
use crate::models::execution::{Contingent, ExecutionData, Fee, Triggers};
use crate::models::instrument::{AssetInfo, InstrumentData, PairInfo};
use crate::models::orders::{OrderEntry, OrdersData};
use crate::models::ticker::TickerData;
use crate::models::trade::TradeData;

// ---------------------------------------------------------------------------
// Python-facing order parameter types (no auth token)
// ---------------------------------------------------------------------------

/// Parameters for placing a new order.
#[pyclass(frozen, get_all, from_py_object)]
#[derive(Debug, Clone)]
pub struct AddOrder {
    pub order_type: OrderType,
    pub side: OrderSide,
    pub symbol: String,
    pub order_qty: Decimal,
    pub limit_price: Option<Decimal>,
    pub time_in_force: Option<TimeInForce>,
    pub expire_time: Option<String>,
    pub post_only: Option<bool>,
    pub reduce_only: Option<bool>,
    pub margin: Option<bool>,
    pub cl_ord_id: Option<String>,
    pub order_userref: Option<i64>,
    pub validate: Option<bool>,
    pub triggers: Option<TriggerParams>,
    pub conditional: Option<ConditionalOrder>,
    pub display_qty: Option<Decimal>,
    pub stp_type: Option<StpType>,
    pub fee_preference: Option<FeeCurrencyPreference>,
    pub no_mpp: Option<bool>,
}

#[pymethods]
impl AddOrder {
    #[new]
    #[pyo3(signature = (
        order_type,
        side,
        symbol,
        order_qty,
        *,
        limit_price = None,
        time_in_force = None,
        expire_time = None,
        post_only = None,
        reduce_only = None,
        margin = None,
        cl_ord_id = None,
        order_userref = None,
        validate = None,
        triggers = None,
        conditional = None,
        display_qty = None,
        stp_type = None,
        fee_preference = None,
        no_mpp = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        order_type: OrderType,
        side: OrderSide,
        symbol: String,
        order_qty: Decimal,
        limit_price: Option<Decimal>,
        time_in_force: Option<TimeInForce>,
        expire_time: Option<String>,
        post_only: Option<bool>,
        reduce_only: Option<bool>,
        margin: Option<bool>,
        cl_ord_id: Option<String>,
        order_userref: Option<i64>,
        validate: Option<bool>,
        triggers: Option<TriggerParams>,
        conditional: Option<ConditionalOrder>,
        display_qty: Option<Decimal>,
        stp_type: Option<StpType>,
        fee_preference: Option<FeeCurrencyPreference>,
        no_mpp: Option<bool>,
    ) -> Self {
        Self {
            order_type,
            side,
            symbol,
            order_qty,
            limit_price,
            time_in_force,
            expire_time,
            post_only,
            reduce_only,
            margin,
            cl_ord_id,
            order_userref,
            validate,
            triggers,
            conditional,
            display_qty,
            stp_type,
            fee_preference,
            no_mpp,
        }
    }
}

/// Parameters for cancelling orders.
#[pyclass(frozen, get_all, from_py_object)]
#[derive(Debug, Clone)]
pub struct CancelOrder {
    pub order_id: Option<Vec<String>>,
    pub cl_ord_id: Option<Vec<String>>,
    pub order_userref: Option<Vec<i64>>,
}

#[pymethods]
impl CancelOrder {
    #[new]
    #[pyo3(signature = (*, order_id = None, cl_ord_id = None, order_userref = None))]
    fn new(
        order_id: Option<Vec<String>>,
        cl_ord_id: Option<Vec<String>>,
        order_userref: Option<Vec<i64>>,
    ) -> Self {
        Self {
            order_id,
            cl_ord_id,
            order_userref,
        }
    }
}

/// Parameters for amending an order (preserves queue priority).
#[pyclass(frozen, get_all, from_py_object)]
#[derive(Debug, Clone)]
pub struct AmendOrder {
    pub order_id: Option<String>,
    pub cl_ord_id: Option<String>,
    pub order_qty: Option<Decimal>,
    pub limit_price: Option<Decimal>,
    pub limit_price_type: Option<PriceType>,
    pub display_qty: Option<Decimal>,
    pub post_only: Option<bool>,
    pub trigger_price: Option<Decimal>,
    pub trigger_price_type: Option<PriceType>,
    pub symbol: Option<String>,
    pub deadline: Option<String>,
}

#[pymethods]
impl AmendOrder {
    #[new]
    #[pyo3(signature = (
        *,
        order_id = None,
        cl_ord_id = None,
        order_qty = None,
        limit_price = None,
        limit_price_type = None,
        display_qty = None,
        post_only = None,
        trigger_price = None,
        trigger_price_type = None,
        symbol = None,
        deadline = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        order_id: Option<String>,
        cl_ord_id: Option<String>,
        order_qty: Option<Decimal>,
        limit_price: Option<Decimal>,
        limit_price_type: Option<PriceType>,
        display_qty: Option<Decimal>,
        post_only: Option<bool>,
        trigger_price: Option<Decimal>,
        trigger_price_type: Option<PriceType>,
        symbol: Option<String>,
        deadline: Option<String>,
    ) -> Self {
        Self {
            order_id,
            cl_ord_id,
            order_qty,
            limit_price,
            limit_price_type,
            display_qty,
            post_only,
            trigger_price,
            trigger_price_type,
            symbol,
            deadline,
        }
    }
}

/// Parameters for editing an order (cancels and replaces).
#[pyclass(frozen, get_all, from_py_object)]
#[derive(Debug, Clone)]
pub struct EditOrder {
    pub order_id: String,
    pub symbol: String,
    pub order_qty: Option<Decimal>,
    pub limit_price: Option<Decimal>,
    pub display_qty: Option<Decimal>,
    pub post_only: Option<bool>,
    pub reduce_only: Option<bool>,
    pub fee_preference: Option<FeeCurrencyPreference>,
    pub order_userref: Option<i64>,
    pub deadline: Option<String>,
    pub triggers: Option<TriggerParams>,
    pub validate: Option<bool>,
}

#[pymethods]
impl EditOrder {
    #[new]
    #[pyo3(signature = (
        order_id,
        symbol,
        *,
        order_qty = None,
        limit_price = None,
        display_qty = None,
        post_only = None,
        reduce_only = None,
        fee_preference = None,
        order_userref = None,
        deadline = None,
        triggers = None,
        validate = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        order_id: String,
        symbol: String,
        order_qty: Option<Decimal>,
        limit_price: Option<Decimal>,
        display_qty: Option<Decimal>,
        post_only: Option<bool>,
        reduce_only: Option<bool>,
        fee_preference: Option<FeeCurrencyPreference>,
        order_userref: Option<i64>,
        deadline: Option<String>,
        triggers: Option<TriggerParams>,
        validate: Option<bool>,
    ) -> Self {
        Self {
            order_id,
            symbol,
            order_qty,
            limit_price,
            display_qty,
            post_only,
            reduce_only,
            fee_preference,
            order_userref,
            deadline,
            triggers,
            validate,
        }
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

#[pymodule]
fn leeson(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Market data
    m.add_class::<TickerData>()?;
    m.add_class::<BookData>()?;
    m.add_class::<PriceLevel>()?;
    m.add_class::<BookDepth>()?;
    m.add_class::<CandleData>()?;
    m.add_class::<TradeData>()?;
    m.add_class::<OrdersData>()?;
    m.add_class::<OrderEntry>()?;

    // Instruments
    m.add_class::<InstrumentData>()?;
    m.add_class::<AssetInfo>()?;
    m.add_class::<PairInfo>()?;

    // Account
    m.add_class::<BalanceData>()?;
    m.add_class::<WalletBalance>()?;
    m.add_class::<BalanceUpdateData>()?;

    // Executions
    m.add_class::<ExecutionData>()?;
    m.add_class::<Fee>()?;
    m.add_class::<Triggers>()?;
    m.add_class::<Contingent>()?;

    // Trading enums
    m.add_class::<OrderType>()?;
    m.add_class::<OrderSide>()?;
    m.add_class::<TimeInForce>()?;
    m.add_class::<TriggerReference>()?;
    m.add_class::<TriggerPriceType>()?;
    m.add_class::<StpType>()?;
    m.add_class::<FeeCurrencyPreference>()?;
    m.add_class::<PriceType>()?;
    m.add_class::<Channel>()?;

    // Order support types
    m.add_class::<TriggerParams>()?;
    m.add_class::<ConditionalOrder>()?;

    // Order parameter types (Python-facing)
    m.add_class::<AddOrder>()?;
    m.add_class::<CancelOrder>()?;
    m.add_class::<AmendOrder>()?;
    m.add_class::<EditOrder>()?;

    // Order responses
    m.add_class::<AddOrderResult>()?;
    m.add_class::<AddOrderResponse>()?;
    m.add_class::<CancelOrderResult>()?;
    m.add_class::<CancelOrderResponse>()?;
    m.add_class::<EditOrderResult>()?;
    m.add_class::<EditOrderResponse>()?;
    m.add_class::<AmendOrderResult>()?;
    m.add_class::<AmendOrderResponse>()?;
    m.add_class::<BatchAddOrderResult>()?;
    m.add_class::<BatchAddResponse>()?;
    m.add_class::<BatchCancelResult>()?;
    m.add_class::<BatchCancelResponse>()?;
    m.add_class::<CancelAllResult>()?;
    m.add_class::<CancelAllResponse>()?;
    m.add_class::<CancelAfterResult>()?;
    m.add_class::<CancelAfterResponse>()?;

    Ok(())
}
