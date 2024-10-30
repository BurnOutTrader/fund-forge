use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};
use crate::oanda_api::models::account::enums::AccountFinancingMode;
use crate::oanda_api::models::order::order_related::TimeInForce;
use crate::oanda_api::models::primitives::{DateTime, HomeConversionFactors, InstrumentName};
use crate::oanda_api::models::trade::TradeID;

pub type TransactionID = String;

#[derive(Serialize, Deserialize, Debug)]
pub enum TransactionType {
    #[serde(rename = "CREATE")]
    Create,
    #[serde(rename = "CLOSE")]
    Close,
    #[serde(rename = "REOPEN")]
    Reopen,
    #[serde(rename = "CLIENT_CONFIGURE")]
    ClientConfigure,
    #[serde(rename = "CLIENT_CONFIGURE_REJECT")]
    ClientConfigureReject,
    #[serde(rename = "TRANSFER_FUNDS")]
    TransferFunds,
    #[serde(rename = "TRANSFER_FUNDS_REJECT")]
    TransferFundsReject,
    #[serde(rename = "MARKET_ORDER")]
    MarketOrder,
    #[serde(rename = "MARKET_ORDER_REJECT")]
    MarketOrderReject,
    #[serde(rename = "FIXED_PRICE_ORDER")]
    FixedPriceOrder,
    #[serde(rename = "LIMIT_ORDER")]
    LimitOrder,
    #[serde(rename = "LIMIT_ORDER_REJECT")]
    LimitOrderReject,
    #[serde(rename = "STOP_ORDER")]
    StopOrder,
    #[serde(rename = "STOP_ORDER_REJECT")]
    StopOrderReject,
    #[serde(rename = "MARKET_IF_TOUCHED_ORDER")]
    MarketIfTouchedOrder,
    #[serde(rename = "MARKET_IF_TOUCHED_ORDER_REJECT")]
    MarketIfTouchedOrderReject,
    #[serde(rename = "TAKE_PROFIT_ORDER")]
    TakeProfitOrder,
    #[serde(rename = "TAKE_PROFIT_ORDER_REJECT")]
    TakeProfitOrderReject,
    #[serde(rename = "STOP_LOSS_ORDER")]
    StopLossOrder,
    #[serde(rename = "STOP_LOSS_ORDER_REJECT")]
    StopLossOrderReject,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER")]
    GuaranteedStopLossOrder,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_REJECT")]
    GuaranteedStopLossOrderReject,
    #[serde(rename = "TRAILING_STOP_LOSS_ORDER")]
    TrailingStopLossOrder,
    #[serde(rename = "TRAILING_STOP_LOSS_ORDER_REJECT")]
    TrailingStopLossOrderReject,
    #[serde(rename = "ORDER_FILL")]
    OrderFill,
    #[serde(rename = "ORDER_CANCEL")]
    OrderCancel,
    #[serde(rename = "ORDER_CANCEL_REJECT")]
    OrderCancelReject,
    #[serde(rename = "ORDER_CLIENT_EXTENSIONS_MODIFY")]
    OrderClientExtensionsModify,
    #[serde(rename = "ORDER_CLIENT_EXTENSIONS_MODIFY_REJECT")]
    OrderClientExtensionsModifyReject,
    #[serde(rename = "TRADE_CLIENT_EXTENSIONS_MODIFY")]
    TradeClientExtensionsModify,
    #[serde(rename = "TRADE_CLIENT_EXTENSIONS_MODIFY_REJECT")]
    TradeClientExtensionsModifyReject,
    #[serde(rename = "MARGIN_CALL_ENTER")]
    MarginCallEnter,
    #[serde(rename = "MARGIN_CALL_EXTEND")]
    MarginCallExtend,
    #[serde(rename = "MARGIN_CALL_EXIT")]
    MarginCallExit,
    #[serde(rename = "DELAYED_TRADE_CLOSURE")]
    DelayedTradeClosure,
    #[serde(rename = "DAILY_FINANCING")]
    DailyFinancing,
    #[serde(rename = "DIVIDEND_ADJUSTMENT")]
    DividendAdjustment,
    #[serde(rename = "RESET_RESETTABLE_PL")]
    ResetResettablePl,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FundingReason {
    #[serde(rename = "CLIENT_FUNDING")]
    ClientFunding,
    #[serde(rename = "ACCOUNT_TRANSFER")]
    AccountTransfer,
    #[serde(rename = "DIVISION_MIGRATION")]
    DivisionMigration,
    #[serde(rename = "SITE_MIGRATION")]
    SiteMigration,
    #[serde(rename = "ADJUSTMENT")]
    Adjustment,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MarketOrderReason {
    #[serde(rename = "CLIENT_ORDER")]
    ClientOrder,
    #[serde(rename = "TRADE_CLOSE")]
    TradeClose,
    #[serde(rename = "POSITION_CLOSEOUT")]
    PositionCloseout,
    #[serde(rename = "MARGIN_CLOSEOUT")]
    MarginCloseout,
    #[serde(rename = "DELAYED_TRADE_CLOSE")]
    DelayedTradeClose,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FixedPriceOrderReason {
    #[serde(rename = "PLATFORM_ACCOUNT_MIGRATION")]
    PlatformAccountMigration,
    #[serde(rename = "TRADE_CLOSE_DIVISION_ACCOUNT_MIGRATION")]
    TradeCloseDivisionAccountMigration,
    #[serde(rename = "TRADE_CLOSE_ADMINISTRATIVE_ACTION")]
    TradeCloseAdministrativeAction,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LimitOrderReason {
    #[serde(rename = "CLIENT_ORDER")]
    ClientOrder,
    #[serde(rename = "REPLACEMENT")]
    Replacement,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StopOrderReason {
    #[serde(rename = "CLIENT_ORDER")]
    ClientOrder,
    #[serde(rename = "REPLACEMENT")]
    Replacement,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TakeProfitOrderReason {
    #[serde(rename = "CLIENT_ORDER")]
    ClientOrder,
    #[serde(rename = "REPLACEMENT")]
    Replacement,
    #[serde(rename = "ON_FILL")]
    OnFill,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StopLossOrderReason {
    #[serde(rename = "CLIENT_ORDER")]
    ClientOrder,
    #[serde(rename = "REPLACEMENT")]
    Replacement,
    #[serde(rename = "ON_FILL")]
    OnFill,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum GuaranteedStopLossOrderReason {
    #[serde(rename = "CLIENT_ORDER")]
    ClientOrder,
    #[serde(rename = "REPLACEMENT")]
    Replacement,
    #[serde(rename = "ON_FILL")]
    OnFill,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TrailingStopLossOrderReason {
    #[serde(rename = "CLIENT_ORDER")]
    ClientOrder,
    #[serde(rename = "REPLACEMENT")]
    Replacement,
    #[serde(rename = "ON_FILL")]
    OnFill,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum OrderFillReason {
    #[serde(rename = "LIMIT_ORDER")]
    LimitOrder,
    #[serde(rename = "STOP_ORDER")]
    StopOrder,
    #[serde(rename = "MARKET_IF_TOUCHED_ORDER")]
    MarketIfTouchedOrder,
    #[serde(rename = "TAKE_PROFIT_ORDER")]
    TakeProfitOrder,
    #[serde(rename = "STOP_LOSS_ORDER")]
    StopLossOrder,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER")]
    GuaranteedStopLossOrder,
    #[serde(rename = "TRAILING_STOP_LOSS_ORDER")]
    TrailingStopLossOrder,
    #[serde(rename = "MARKET_ORDER")]
    MarketOrder,
    #[serde(rename = "MARKET_ORDER_TRADE_CLOSE")]
    MarketOrderTradeClose,
    #[serde(rename = "MARKET_ORDER_POSITION_CLOSEOUT")]
    MarketOrderPositionCloseout,
    #[serde(rename = "MARKET_ORDER_MARGIN_CLOSEOUT")]
    MarketOrderMarginCloseout,
    #[serde(rename = "MARKET_ORDER_DELAYED_TRADE_CLOSE")]
    MarketOrderDelayedTradeClose,
    #[serde(rename = "FIXED_PRICE_ORDER")]
    FixedPriceOrder,
    #[serde(rename = "FIXED_PRICE_ORDER_PLATFORM_ACCOUNT_MIGRATION")]
    FixedPriceOrderPlatformAccountMigration,
    #[serde(rename = "FIXED_PRICE_ORDER_DIVISION_ACCOUNT_MIGRATION")]
    FixedPriceOrderDivisionAccountMigration,
    #[serde(rename = "FIXED_PRICE_ORDER_ADMINISTRATIVE_ACTION")]
    FixedPriceOrderAdministrativeAction,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum OrderCancelReason {
    #[serde(rename = "INTERNAL_SERVER_ERROR")]
    InternalServerError,
    #[serde(rename = "ACCOUNT_LOCKED")]
    AccountLocked,
    #[serde(rename = "ACCOUNT_NEW_POSITIONS_LOCKED")]
    AccountNewPositionsLocked,
    #[serde(rename = "ACCOUNT_ORDER_CREATION_LOCKED")]
    AccountOrderCreationLocked,
    #[serde(rename = "ACCOUNT_ORDER_FILL_LOCKED")]
    AccountOrderFillLocked,
    #[serde(rename = "CLIENT_REQUEST")]
    ClientRequest,
    #[serde(rename = "MIGRATION")]
    Migration,
    #[serde(rename = "MARKET_HALTED")]
    MarketHalted,
    #[serde(rename = "LINKED_TRADE_CLOSED")]
    LinkedTradeClosed,
    #[serde(rename = "TIME_IN_FORCE_EXPIRED")]
    TimeInForceExpired,
    #[serde(rename = "INSUFFICIENT_MARGIN")]
    InsufficientMargin,
    #[serde(rename = "FIFO_VIOLATION")]
    FifoViolation,
    #[serde(rename = "BOUNDS_VIOLATION")]
    BoundsViolation,
    #[serde(rename = "CLIENT_REQUEST_REPLACED")]
    ClientRequestReplaced,
    #[serde(rename = "DIVIDEND_ADJUSTMENT_REPLACED")]
    DividendAdjustmentReplaced,
    #[serde(rename = "INSUFFICIENT_LIQUIDITY")]
    InsufficientLiquidity,
    #[serde(rename = "TAKE_PROFIT_ON_FILL_GTD_TIMESTAMP_IN_PAST")]
    TakeProfitOnFillGtdTimestampInPast,
    #[serde(rename = "TAKE_PROFIT_ON_FILL_LOSS")]
    TakeProfitOnFillLoss,
    #[serde(rename = "LOSING_TAKE_PROFIT")]
    LosingTakeProfit,
    #[serde(rename = "STOP_LOSS_ON_FILL_GTD_TIMESTAMP_IN_PAST")]
    StopLossOnFillGtdTimestampInPast,
    #[serde(rename = "STOP_LOSS_ON_FILL_LOSS")]
    StopLossOnFillLoss,
    #[serde(rename = "STOP_LOSS_ON_FILL_PRICE_DISTANCE_MAXIMUM_EXCEEDED")]
    StopLossOnFillPriceDistanceMaximumExceeded,
    #[serde(rename = "STOP_LOSS_ON_FILL_REQUIRED")]
    StopLossOnFillRequired,
    #[serde(rename = "STOP_LOSS_ON_FILL_GUARANTEED_REQUIRED")]
    StopLossOnFillGuaranteedRequired,
    #[serde(rename = "STOP_LOSS_ON_FILL_GUARANTEED_NOT_ALLOWED")]
    StopLossOnFillGuaranteedNotAllowed,
    #[serde(rename = "STOP_LOSS_ON_FILL_GUARANTEED_MINIMUM_DISTANCE_NOT_MET")]
    StopLossOnFillGuaranteedMinimumDistanceNotMet,
    #[serde(rename = "STOP_LOSS_ON_FILL_GUARANTEED_LEVEL_RESTRICTION_EXCEEDED")]
    StopLossOnFillGuaranteedLevelRestrictionExceeded,
    #[serde(rename = "STOP_LOSS_ON_FILL_GUARANTEED_HEDGING_NOT_ALLOWED")]
    StopLossOnFillGuaranteedHedgingNotAllowed,
    #[serde(rename = "STOP_LOSS_ON_FILL_TIME_IN_FORCE_INVALID")]
    StopLossOnFillTimeInForceInvalid,
    #[serde(rename = "STOP_LOSS_ON_FILL_TRIGGER_CONDITION_INVALID")]
    StopLossOnFillTriggerConditionInvalid,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_GTD_TIMESTAMP_IN_PAST")]
    GuaranteedStopLossOnFillGtdTimestampInPast,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_LOSS")]
    GuaranteedStopLossOnFillLoss,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_PRICE_DISTANCE_MAXIMUM_EXCEEDED")]
    GuaranteedStopLossOnFillPriceDistanceMaximumExceeded,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_REQUIRED")]
    GuaranteedStopLossOnFillRequired,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_NOT_ALLOWED")]
    GuaranteedStopLossOnFillNotAllowed,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_MINIMUM_DISTANCE_NOT_MET")]
    GuaranteedStopLossOnFillMinimumDistanceNotMet,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_LEVEL_RESTRICTION_VOLUME_EXCEEDED")]
    GuaranteedStopLossOnFillLevelRestrictionVolumeExceeded,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_LEVEL_RESTRICTION_PRICE_RANGE_EXCEEDED")]
    GuaranteedStopLossOnFillLevelRestrictionPriceRangeExceeded,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_HEDGING_NOT_ALLOWED")]
    GuaranteedStopLossOnFillHedgingNotAllowed,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_TIME_IN_FORCE_INVALID")]
    GuaranteedStopLossOnFillTimeInForceInvalid,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_TRIGGER_CONDITION_INVALID")]
    GuaranteedStopLossOnFillTriggerConditionInvalid,
    #[serde(rename = "TAKE_PROFIT_ON_FILL_PRICE_DISTANCE_MAXIMUM_EXCEEDED")]
    TakeProfitOnFillPriceDistanceMaximumExceeded,
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_GTD_TIMESTAMP_IN_PAST")]
    TrailingStopLossOnFillGtdTimestampInPast,
    #[serde(rename = "CLIENT_TRADE_ID_ALREADY_EXISTS")]
    ClientTradeIdAlreadyExists,
    #[serde(rename = "POSITION_CLOSEOUT_FAILED")]
    PositionCloseoutFailed,
    #[serde(rename = "OPEN_TRADES_ALLOWED_EXCEEDED")]
    OpenTradesAllowedExceeded,
    #[serde(rename = "PENDING_ORDERS_ALLOWED_EXCEEDED")]
    PendingOrdersAllowedExceeded,
    #[serde(rename = "TAKE_PROFIT_ON_FILL_CLIENT_ORDER_ID_ALREADY_EXISTS")]
    TakeProfitOnFillClientOrderIdAlreadyExists,
    #[serde(rename = "STOP_LOSS_ON_FILL_CLIENT_ORDER_ID_ALREADY_EXISTS")]
    StopLossOnFillClientOrderIdAlreadyExists,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_CLIENT_ORDER_ID_ALREADY_EXISTS")]
    GuaranteedStopLossOnFillClientOrderIdAlreadyExists,
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_CLIENT_ORDER_ID_ALREADY_EXISTS")]
    TrailingStopLossOnFillClientOrderIdAlreadyExists,
    #[serde(rename = "POSITION_SIZE_EXCEEDED")]
    PositionSizeExceeded,
    #[serde(rename = "HEDGING_GSLO_VIOLATION")]
    HedgingGsloViolation,
    #[serde(rename = "ACCOUNT_POSITION_VALUE_LIMIT_EXCEEDED")]
    AccountPositionValueLimitExceeded,
    #[serde(rename = "INSTRUMENT_BID_REDUCE_ONLY")]
    InstrumentBidReduceOnly,
    #[serde(rename = "INSTRUMENT_ASK_REDUCE_ONLY")]
    InstrumentAskReduceOnly,
    #[serde(rename = "INSTRUMENT_BID_HALTED")]
    InstrumentBidHalted,
    #[serde(rename = "INSTRUMENT_ASK_HALTED")]
    InstrumentAskHalted,
    #[serde(rename = "STOP_LOSS_ON_FILL_GUARANTEED_BID_HALTED")]
    StopLossOnFillGuaranteedBidHalted,
    #[serde(rename = "STOP_LOSS_ON_FILL_GUARANTEED_ASK_HALTED")]
    StopLossOnFillGuaranteedAskHalted,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_BID_HALTED")]
    GuaranteedStopLossOnFillBidHalted,
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_ASK_HALTED")]
    GuaranteedStopLossOnFillAskHalted,
    #[serde(rename = "FIFO_VIOLATION_SAFEGUARD_VIOLATION")]
    FifoViolationSafeguardViolation,
    #[serde(rename = "FIFO_VIOLATION_SAFEGUARD_PARTIAL_CLOSE_VIOLATION")]
    FifoViolationSafeguardPartialCloseViolation,
    #[serde(rename = "ORDERS_ON_FILL_RMO_MUTUAL_EXCLUSIVITY_MUTUALLY_EXCLUSIVE_VIOLATION")]
    OrdersOnFillRmoMutualExclusivityMutuallyExclusiveViolation,
}

/// Represents the dividend adjustment for an open trade within the account.
#[derive(Serialize, Deserialize, Debug)]
pub struct OpenTradeDividendAdjustment {
    /// The ID of the trade for which the dividend adjustment is to be paid or collected.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The dividend adjustment amount to pay or collect for the trade.
    #[serde(rename = "dividendAdjustment")]
    pub dividend_adjustment: Decimal,

    /// The dividend adjustment amount to pay or collect for the trade, in the instrument's quote currency.
    #[serde(rename = "quoteDividendAdjustment")]
    pub quote_dividend_adjustment: Decimal,
}

/// A client-provided identifier used to refer to orders or trades.
pub(crate) type ClientID = String;

/// A client-provided tag that can contain any data and may be assigned to orders or trades.
pub type ClientTag = String;

/// A client-provided comment that can contain any data and may be assigned to orders or trades.
pub type ClientComment = String;

/// ClientExtensions allow clients to attach a clientID, tag, and comment to orders and trades.
#[derive(Serialize, Deserialize, Debug)]
pub struct ClientExtensions {
    /// The client ID of the order/trade.
    pub id: ClientID,

    /// A tag associated with the order/trade.
    pub tag: ClientTag,

    /// A comment associated with the order/trade.
    pub comment: ClientComment,
}

/// Specifies the details of a Take Profit Order to be created on behalf of a client.
#[derive(Serialize, Deserialize, Debug)]
pub struct TakeProfitDetails {
    /// The prices at which the Take Profit Order will be triggered.
    /// Only one of the prices and distance fields may be specified.
    pub price: Decimal,

    /// The time in force for the created Take Profit Order (GTC, GTD, or GFD).
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date when the Take Profit Order will be cancelled if timeInForce is GTD.
    #[serde(rename = "gtdTime")]
    pub gtd_time: DateTime,

    /// The client extensions to add to the Take Profit Order when created.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: ClientExtensions,
}

/// Specifies the details of a Stop Loss Order to be created on behalf of a client.
#[derive(Serialize, Deserialize, Debug)]
pub struct StopLossDetails {
    /// The prices at which the Stop Loss Order will be triggered.
    /// Only one of the prices and distance fields may be specified.
    pub price: Decimal,

    /// Specifies the distance (in prices units) from the Trade’s open prices to
    /// use as the Stop Loss Order prices.
    /// Only one of the distance and prices fields may be specified.
    pub distance: Decimal,

    /// The time in force for the created Stop Loss Order (GTC, GTD, or GFD).
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date when the Stop Loss Order will be cancelled if timeInForce is GTD.
    #[serde(rename = "gtdTime")]
    pub gtd_time: DateTime,

    /// The Client Extensions to add to the Stop Loss Order when created.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: ClientExtensions,

    /// Flag indicating that the prices for the Stop Loss Order is guaranteed.
    /// The default value depends on the GuaranteedStopLossOrderMode of the account.
    /// If it is REQUIRED, the default will be true; for DISABLED or ENABLED, the default is false.
    /// Deprecated: Will be removed in a future API update.
    #[serde(rename = "guaranteed", default)]
    pub guaranteed: bool,
}

/// Specifies the details of a Guaranteed Stop Loss Order to be created on behalf of a client.
#[derive(Serialize, Deserialize, Debug)]
pub struct GuaranteedStopLossDetails {
    /// The prices at which the Guaranteed Stop Loss Order will be triggered.
    /// Only one of the prices and distance fields may be specified.
    pub price: Decimal,

    /// Specifies the distance (in prices units) from the Trade’s open prices to
    /// use as the Guaranteed Stop Loss Order prices.
    /// Only one of the distance and prices fields may be specified.
    pub distance: Decimal,

    /// The time in force for the created Guaranteed Stop Loss Order (GTC, GTD, or GFD).
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date when the Guaranteed Stop Loss Order will be cancelled if timeInForce is GTD.
    #[serde(rename = "gtdTime")]
    pub gtd_time: DateTime,

    /// The Client Extensions to add to the Guaranteed Stop Loss Order when created.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: ClientExtensions,
}

/// Specifies the details of a Trailing Stop Loss Order to be created on behalf of a client.
#[derive(Serialize, Deserialize, Debug)]
pub struct TrailingStopLossDetails {
    /// The distance (in prices units) from the Trade’s fill prices that the
    /// Trailing Stop Loss Order will be triggered at.
    pub distance: Decimal,

    /// The time in force for the created Trailing Stop Loss Order (GTC, GTD, or GFD).
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,

    /// The date when the Trailing Stop Loss Order will be cancelled if timeInForce is GTD.
    #[serde(rename = "gtdTime")]
    pub gtd_time: DateTime,

    /// The Client Extensions to add to the Trailing Stop Loss Order when created.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: ClientExtensions,
}

/// Represents a Trade for an instrument that was opened in an Account.
#[derive(Serialize, Deserialize, Debug)]
pub struct TradeOpen {
    /// The ID of the Trade that was opened.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The number of units opened by the Trade.
    pub units: Decimal,

    /// The average prices that the units were opened at.
    pub price: Decimal,

    /// The fee charged for opening the trade if it has a guaranteed Stop Loss Order attached to it.
    #[serde(rename = "guaranteedExecutionFee")]
    pub guaranteed_execution_fee: Decimal,

    /// The fee charged for opening the trade if it has a guaranteed Stop Loss Order attached to it, expressed in the Instrument’s quote currency.
    #[serde(rename = "quoteGuaranteedExecutionFee")]
    pub quote_guaranteed_execution_fee: Decimal,

    /// The client extensions for the newly opened Trade.
    #[serde(rename = "clientExtensions")]
    pub client_extensions: ClientExtensions,

    /// The half spread cost for the trade open. This can be a positive or negative value and is represented in the home currency of the Account.
    #[serde(rename = "halfSpreadCost")]
    pub half_spread_cost: Decimal,

    /// The margin required at the time the Trade was created. Note, this is the ‘pure’ margin required, it is not the ‘effective’ margin used that factors in the trade risk if a GSLO is attached to the trade.
    #[serde(rename = "initialMarginRequired")]
    pub initial_margin_required: Decimal,
}

/// Represents a Trade for an instrument that was reduced (either partially or fully) in an Account.
#[derive(Serialize, Deserialize, Debug)]
pub struct TradeReduce {
    /// The ID of the Trade that was reduced or closed.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The number of units that the Trade was reduced by.
    pub units: Decimal,

    /// The average prices that the units were closed at. This prices may be clamped for guaranteed Stop Loss Orders.
    pub price: Decimal,

    /// The PL realized when reducing the Trade.
    #[serde(rename = "realizedPL")]
    pub realized_pl: Decimal,

    /// The financing paid/collected when reducing the Trade.
    pub financing: Decimal,

    /// The base financing paid/collected when reducing the Trade.
    #[serde(rename = "baseFinancing")]
    pub base_financing: Decimal,

    /// The quote financing paid/collected when reducing the Trade.
    #[serde(rename = "quoteFinancing")]
    pub quote_financing: Decimal,

    /// The financing rate in effect for the instrument used to calculate the amount of financing paid/collected when reducing the Trade. This field will only be set if the AccountFinancingMode at the time of the order fill is SECOND_BY_SECOND_INSTRUMENT. The value is in decimal rather than percentage points, e.g. 5% is represented as 0.05.
    #[serde(rename = "financingRate")]
    pub financing_rate: Decimal,

    /// The fee that is charged for closing the Trade if it has a guaranteed Stop Loss Order attached to it.
    #[serde(rename = "guaranteedExecutionFee")]
    pub guaranteed_execution_fee: Decimal,

    /// The fee that is charged for closing the Trade if it has a guaranteed Stop Loss Order attached to it, expressed in the Instrument’s quote currency.
    #[serde(rename = "quoteGuaranteedExecutionFee")]
    pub quote_guaranteed_execution_fee: Decimal,

    /// The half spread cost for the trade reduce/close. This can be a positive or negative value and is represented in the home currency of the Account.
    #[serde(rename = "halfSpreadCost")]
    pub half_spread_cost: Decimal,
}

/// Specifies the extensions to a Market Order that has been created specifically to close a Trade.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarketOrderTradeClose {
    /// The ID of the Trade requested to be closed.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The client ID of the Trade requested to be closed.
    #[serde(rename = "clientTradeID")]
    pub client_trade_id: String,

    /// Indication of how much of the Trade to close. Either “ALL”, or a Decimal reflecting a partial close of the Trade.
    pub units: String,
}

/// The reason that the Market Order was created to perform a margin closeout.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketOrderMarginCloseoutReason {
    /// Trade closures resulted from violating OANDA's margin policy.
    MarginCheckViolation,

    /// Trade closures came from a margin closeout event resulting from regulatory conditions placed on the Account's margin call.
    RegulatoryMarginCallViolation,

    /// Trade closures resulted from violating the margin policy imposed by regulatory requirements.
    RegulatoryMarginCheckViolation,
}

/// Details for the Market Order extensions specific to a Market Order placed with the intent of fully closing a specific open trade that should have already been closed but wasn’t due to halted market conditions.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarketOrderDelayedTradeClose {
    /// The ID of the Trade being closed.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The Client ID of the Trade being closed.
    #[serde(rename = "clientTradeID")]
    pub client_trade_id: TradeID,

    /// The Transaction ID of the DelayedTradeClosure transaction to which this Delayed Trade Close belongs to.
    #[serde(rename = "sourceTransactionID")]
    pub source_transaction_id: TransactionID,
}

/// Specifies the extensions to a Market Order when it has been created to closeout a specific Position.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarketOrderPositionCloseout {
    /// The instrument of the Position being closed out.
    pub instrument: InstrumentName,

    /// Indication of how much of the Position to close. Either “ALL”, or a Decimal reflecting a partial close of the Trade. The Decimal must always be positive and represent a number that doesn’t exceed the absolute size of the Position.
    pub units: String,
}

/// A LiquidityRegenerationSchedule indicates how liquidity that is used when filling an Order for an instrument is regenerated following the fill. A liquidity regeneration schedule will be in effect until the timestamp of its final step, but may be replaced by a schedule created for an Order of the same instrument that is filled while it is still in effect.
#[derive(Serialize, Deserialize, Debug)]
pub struct LiquidityRegenerationSchedule {
    /// The steps in the Liquidity Regeneration Schedule.
    pub steps: Vec<LiquidityRegenerationScheduleStep>,
}

/// A liquidity regeneration schedule Step indicates the amount of bid and ask liquidity that is used by the Account at a certain time. These amounts will only change at the timestamp of the following step.
#[derive(Serialize, Deserialize, Debug)]
pub struct LiquidityRegenerationScheduleStep {
    /// The timestamp of the schedule step.
    pub timestamp: DateTime,

    /// The amount of bid liquidity used at this step in the schedule.
    #[serde(rename = "bidLiquidityUsed")]
    pub bid_liquidity_used: Decimal,

    /// The amount of ask liquidity used at this step in the schedule.
    #[serde(rename = "askLiquidityUsed")]
    pub ask_liquidity_used: Decimal,
}

/// OpenTradeFinancing is used to pay/collect daily financing charge for an open Trade within an Account.
#[derive(Serialize, Deserialize, Debug)]
pub struct OpenTradeFinancing {
    /// The ID of the Trade that financing is being paid/collected for.
    #[serde(rename = "tradeID")]
    pub trade_id: TradeID,

    /// The amount of financing paid/collected for the Trade.
    pub financing: Decimal,

    /// The amount of financing paid/collected in the Instrument’s base currency for the Trade.
    #[serde(rename = "baseFinancing")]
    pub base_financing: Decimal,

    /// The amount of financing paid/collected in the Instrument’s quote currency for the Trade.
    #[serde(rename = "quoteFinancing")]
    pub quote_financing: Decimal,

    /// The financing rate in effect for the instrument used to calculate the amount of financing paid/collected for the Trade. This field will only be set if the AccountFinancingMode at the time of the daily financing is DAILY_INSTRUMENT or SECOND_BY_SECOND_INSTRUMENT. The value is in decimal rather than percentage points, e.g. 5% is represented as 0.05.
    #[serde(rename = "financingRate")]
    pub financing_rate: Decimal,
}

/// PositionFinancing is used to pay/collect daily financing charge for a Position within an Account.
#[derive(Serialize, Deserialize, Debug)]
pub struct PositionFinancing {
    /// The instrument of the Position that financing is being paid/collected for.
    pub instrument: InstrumentName,

    /// The amount of financing paid/collected for the Position.
    pub financing: Decimal,

    /// The amount of base financing paid/collected for the Position.
    #[serde(rename = "baseFinancing")]
    pub base_financing: Decimal,

    /// The amount of quote financing paid/collected for the Position.
    #[serde(rename = "quoteFinancing")]
    pub quote_financing: Decimal,

    /// The HomeConversionFactors in effect for the Position’s Instrument at the time of the DailyFinancing.
    #[serde(rename = "homeConversionFactors")]
    pub home_conversion_factors: HomeConversionFactors,

    /// The financing paid/collected for each open Trade within the Position.
    #[serde(rename = "openTradeFinancings")]
    pub open_trade_financings: Vec<OpenTradeFinancing>,

    /// The account financing mode at the time of the daily financing.
    #[serde(rename = "accountFinancingMode")]
    pub account_financing_mode: AccountFinancingMode,
}

#[allow(dead_code)]
pub(crate) type ClientRequestID = String;
#[allow(dead_code)]
pub(crate) type RequestID = String;

/// TransactionRejectReason represents the reason that a Transaction was rejected.
#[derive(Serialize, Deserialize, Debug)]
pub enum TransactionRejectReason {
    /// An unexpected internal server error has occurred
    #[serde(rename = "INTERNAL_SERVER_ERROR")]
    InternalServerError,

    /// The system was unable to determine the current prices for the Order’s instrument
    #[serde(rename = "INSTRUMENT_PRICE_UNKNOWN")]
    InstrumentPriceUnknown,

    /// The Account is not active
    #[serde(rename = "ACCOUNT_NOT_ACTIVE")]
    AccountNotActive,

    /// The Account is locked
    #[serde(rename = "ACCOUNT_LOCKED")]
    AccountLocked,

    /// The Account is locked for Order creation
    #[serde(rename = "ACCOUNT_ORDER_CREATION_LOCKED")]
    AccountOrderCreationLocked,

    /// The Account is locked for configuration
    #[serde(rename = "ACCOUNT_CONFIGURATION_LOCKED")]
    AccountConfigurationLocked,

    /// The Account is locked for deposits
    #[serde(rename = "ACCOUNT_DEPOSIT_LOCKED")]
    AccountDepositLocked,

    /// The Account is locked for withdrawals
    #[serde(rename = "ACCOUNT_WITHDRAWAL_LOCKED")]
    AccountWithdrawalLocked,

    /// The Account is locked for Order cancellation
    #[serde(rename = "ACCOUNT_ORDER_CANCEL_LOCKED")]
    AccountOrderCancelLocked,

    /// The instrument specified is not tradeable by the Account
    #[serde(rename = "INSTRUMENT_NOT_TRADEABLE")]
    InstrumentNotTradeable,

    /// Creating the Order would result in the maximum number of allowed pending Orders being exceeded
    #[serde(rename = "PENDING_ORDERS_ALLOWED_EXCEEDED")]
    PendingOrdersAllowedExceeded,

    /// Neither the Order ID nor client Order ID are specified
    #[serde(rename = "ORDER_ID_UNSPECIFIED")]
    OrderIdUnspecified,

    /// The Order specified does not exist
    #[serde(rename = "ORDER_DOESNT_EXIST")]
    OrderDoesntExist,

    /// The Order ID and client Order ID specified do not identify the same Order
    #[serde(rename = "ORDER_IDENTIFIER_INCONSISTENCY")]
    OrderIdentifierInconsistency,

    /// Neither the Trade ID nor client Trade ID are specified
    #[serde(rename = "TRADE_ID_UNSPECIFIED")]
    TradeIdUnspecified,

    /// The Trade specified does not exist
    #[serde(rename = "TRADE_DOESNT_EXIST")]
    TradeDoesntExist,

    /// The Trade ID and client Trade ID specified do not identify the same Trade
    #[serde(rename = "TRADE_IDENTIFIER_INCONSISTENCY")]
    TradeIdentifierInconsistency,

    /// The Account had insufficient margin to perform the action specified. One possible reason for this is due to the creation or modification of a guaranteed StopLoss Order.
    #[serde(rename = "INSUFFICIENT_MARGIN")]
    InsufficientMargin,

    /// Order instrument has not been specified
    #[serde(rename = "INSTRUMENT_MISSING")]
    InstrumentMissing,

    /// The instrument specified is unknown
    #[serde(rename = "INSTRUMENT_UNKNOWN")]
    InstrumentUnknown,

    /// Order units have not been specified
    #[serde(rename = "UNITS_MISSING")]
    UnitsMissing,

    /// Order units specified are invalid
    #[serde(rename = "UNITS_INVALID")]
    UnitsInvalid,

    /// The units specified contain more precision than is allowed for the Order’s instrument
    #[serde(rename = "UNITS_PRECISION_EXCEEDED")]
    UnitsPrecisionExceeded,

    /// The units specified exceeds the maximum number of units allowed
    #[serde(rename = "UNITS_LIMIT_EXCEEDED")]
    UnitsLimitExceeded,

    /// The units specified is less than the minimum number of units required
    #[serde(rename = "UNITS_MINIMUM_NOT_MET")]
    UnitsMinimumNotMet,

    /// The prices has not been specified
    #[serde(rename = "PRICE_MISSING")]
    PriceMissing,

    /// The prices specified is invalid
    #[serde(rename = "PRICE_INVALID")]
    PriceInvalid,

    /// The prices specified contains more precision than is allowed for the instrument
    #[serde(rename = "PRICE_PRECISION_EXCEEDED")]
    PricePrecisionExceeded,

    /// The prices distance has not been specified
    #[serde(rename = "PRICE_DISTANCE_MISSING")]
    PriceDistanceMissing,

    /// The prices distance specified is invalid
    #[serde(rename = "PRICE_DISTANCE_INVALID")]
    PriceDistanceInvalid,

    /// The prices distance specified contains more precision than is allowed for the instrument
    #[serde(rename = "PRICE_DISTANCE_PRECISION_EXCEEDED")]
    PriceDistancePrecisionExceeded,

    /// The prices distance exceeds the maximum allowed amount
    #[serde(rename = "PRICE_DISTANCE_MAXIMUM_EXCEEDED")]
    PriceDistanceMaximumExceeded,

    /// The prices distance does not meet the minimum allowed amount
    #[serde(rename = "PRICE_DISTANCE_MINIMUM_NOT_MET")]
    PriceDistanceMinimumNotMet,

    /// The TimeInForce field has not been specified
    #[serde(rename = "TIME_IN_FORCE_MISSING")]
    TimeInForceMissing,

    /// The TimeInForce specified is invalid
    #[serde(rename = "TIME_IN_FORCE_INVALID")]
    TimeInForceInvalid,

    /// The TimeInForce is GTD but no GTD timestamp is provided
    #[serde(rename = "TIME_IN_FORCE_GTD_TIMESTAMP_MISSING")]
    TimeInForceGtdTimestampMissing,

    /// The TimeInForce is GTD but the GTD timestamp is in the past
    #[serde(rename = "TIME_IN_FORCE_GTD_TIMESTAMP_IN_PAST")]
    TimeInForceGtdTimestampInPast,

    /// The prices bound specified is invalid
    #[serde(rename = "PRICE_BOUND_INVALID")]
    PriceBoundInvalid,

    /// The prices bound specified contains more precision than is allowed for the Order’s instrument
    #[serde(rename = "PRICE_BOUND_PRECISION_EXCEEDED")]
    PriceBoundPrecisionExceeded,

    /// Multiple Orders on fill share the same client Order ID
    #[serde(rename = "ORDERS_ON_FILL_DUPLICATE_CLIENT_ORDER_IDS")]
    OrdersOnFillDuplicateClientOrderIds,

    /// The Order does not support Trade on fill client extensions because it cannot create a new Trade
    #[serde(rename = "TRADE_ON_FILL_CLIENT_EXTENSIONS_NOT_SUPPORTED")]
    TradeOnFillClientExtensionsNotSupported,

    /// The client Order ID specified is invalid
    #[serde(rename = "CLIENT_ORDER_ID_INVALID")]
    ClientOrderIdInvalid,

    /// The client Order ID specified is already assigned to another pending Order
    #[serde(rename = "CLIENT_ORDER_ID_ALREADY_EXISTS")]
    ClientOrderIdAlreadyExists,

    /// The client Order tag specified is invalid
    #[serde(rename = "CLIENT_ORDER_TAG_INVALID")]
    ClientOrderTagInvalid,

    /// The client Order comment specified is invalid
    #[serde(rename = "CLIENT_ORDER_COMMENT_INVALID")]
    ClientOrderCommentInvalid,

    /// The client Trade ID specified is invalid
    #[serde(rename = "CLIENT_TRADE_ID_INVALID")]
    ClientTradeIdInvalid,

    /// The client Trade ID specified is already assigned to another open Trade
    #[serde(rename = "CLIENT_TRADE_ID_ALREADY_EXISTS")]
    ClientTradeIdAlreadyExists,

    /// The client Trade tag specified is invalid
    #[serde(rename = "CLIENT_TRADE_TAG_INVALID")]
    ClientTradeTagInvalid,

    /// The client Trade comment is invalid
    #[serde(rename = "CLIENT_TRADE_COMMENT_INVALID")]
    ClientTradeCommentInvalid,

    /// The OrderFillPositionAction field has not been specified
    #[serde(rename = "ORDER_FILL_POSITION_ACTION_MISSING")]
    OrderFillPositionActionMissing,

    /// The OrderFillPositionAction specified is invalid
    #[serde(rename = "ORDER_FILL_POSITION_ACTION_INVALID")]
    OrderFillPositionActionInvalid,

    /// The TriggerCondition field has not been specified
    #[serde(rename = "TRIGGER_CONDITION_MISSING")]
    TriggerConditionMissing,

    /// The TriggerCondition specified is invalid
    #[serde(rename = "TRIGGER_CONDITION_INVALID")]
    TriggerConditionInvalid,

    /// The OrderFillPositionAction field has not been specified
    #[serde(rename = "ORDER_PARTIAL_FILL_OPTION_MISSING")]
    OrderPartialFillOptionMissing,

    /// The OrderFillPositionAction specified is invalid.
    #[serde(rename = "ORDER_PARTIAL_FILL_OPTION_INVALID")]
    OrderPartialFillOptionInvalid,

    /// When attempting to reissue an order (currently only a MarketIfTouched) that was immediately partially filled, it is not possible to create a correct pending Order.
    #[serde(rename = "INVALID_REISSUE_IMMEDIATE_PARTIAL_FILL")]
    InvalidReissueImmediatePartialFill,

    /// The Orders on fill would be in violation of the risk management Order mutual exclusivity configuration specifying that only one risk management Order can be attached to a Trade.
    #[serde(rename = "ORDERS_ON_FILL_RMO_MUTUAL_EXCLUSIVITY_MUTUALLY_EXCLUSIVE_VIOLATION")]
    OrdersOnFillRmoMutualExclusivityMutuallyExclusiveViolation,

    /// The Orders on fill would be in violation of the risk management Order mutual exclusivity configuration specifying that if a GSLO is already attached to a Trade, no other risk management Order can be attached to a Trade.
    #[serde(rename = "ORDERS_ON_FILL_RMO_MUTUAL_EXCLUSIVITY_GSLO_EXCLUDES_OTHERS_VIOLATION")]
    OrdersOnFillRmoMutualExclusivityGsloExcludesOthersViolation,

    /// A Take Profit Order for the specified Trade already exists
    #[serde(rename = "TAKE_PROFIT_ORDER_ALREADY_EXISTS")]
    TakeProfitOrderAlreadyExists,

    /// An attempt was made to create a non-guaranteed stop loss order in an account that requires all stop loss orders to be guaranteed.
    #[serde(rename = "STOP_LOSS_ORDER_GUARANTEED_REQUIRED")]
    StopLossOrderGuaranteedRequired,

    /// An attempt to create a guaranteed stop loss order with a prices that is within the current tradeable spread.
    #[serde(rename = "STOP_LOSS_ORDER_GUARANTEED_PRICE_WITHIN_SPREAD")]
    StopLossOrderGuaranteedPriceWithinSpread,

    /// An attempt was made to create a guaranteed Stop Loss Order, however the Account’s configuration does not allow guaranteed Stop Loss Orders.
    #[serde(rename = "STOP_LOSS_ORDER_GUARANTEED_NOT_ALLOWED")]
    StopLossOrderGuaranteedNotAllowed,

    /// An attempt was made to create a guaranteed Stop Loss Order when the market was halted.
    #[serde(rename = "STOP_LOSS_ORDER_GUARANTEED_HALTED_CREATE_VIOLATION")]
    StopLossOrderGuaranteedHaltedCreateViolation,

    /// An attempt was made to re-create a guaranteed Stop Loss Order with a tighter fill prices when the market was halted.
    #[serde(rename = "STOP_LOSS_ORDER_GUARANTEED_HALTED_TIGHTEN_VIOLATION")]
    StopLossOrderGuaranteedHaltedTightenViolation,

    /// An attempt was made to create a guaranteed Stop Loss Order on a hedged Trade (i.e., there is an existing open Trade in the opposing direction), however the Account’s configuration does not allow guaranteed Stop Loss Orders for hedged Trades/Positions.
    #[serde(rename = "STOP_LOSS_ORDER_GUARANTEED_HEDGING_NOT_ALLOWED")]
    StopLossOrderGuaranteedHedgingNotAllowed,

    /// An attempt was made to create a guaranteed Stop Loss Order, however the distance between the current prices and the trigger prices does not meet the Account’s configured minimum Guaranteed Stop Loss distance.
    #[serde(rename = "STOP_LOSS_ORDER_GUARANTEED_MINIMUM_DISTANCE_NOT_MET")]
    StopLossOrderGuaranteedMinimumDistanceNotMet,

    /// An attempt was made to cancel a Stop Loss Order, however the Account’s configuration requires every Trade to have an associated Stop Loss Order.
    #[serde(rename = "STOP_LOSS_ORDER_NOT_CANCELABLE")]
    StopLossOrderNotCancelable,

    /// An attempt was made to cancel and replace a Stop Loss Order, however the Account’s configuration prevents the modification of Stop Loss Orders.
    #[serde(rename = "STOP_LOSS_ORDER_NOT_REPLACEABLE")]
    StopLossOrderNotReplaceable,

    /// An attempt was made to create a guaranteed Stop Loss Order, however doing so would exceed the Account’s configured guaranteed Stop Loss Order level restriction volume.
    #[serde(rename = "STOP_LOSS_ORDER_GUARANTEED_LEVEL_RESTRICTION_EXCEEDED")]
    StopLossOrderGuaranteedLevelRestrictionExceeded,

    /// The Stop Loss Order request contains both the prices and distance fields.
    #[serde(rename = "STOP_LOSS_ORDER_PRICE_AND_DISTANCE_BOTH_SPECIFIED")]
    StopLossOrderPriceAndDistanceBothSpecified,

    /// The Stop Loss Order request contains neither the prices nor distance fields.
    #[serde(rename = "STOP_LOSS_ORDER_PRICE_AND_DISTANCE_BOTH_MISSING")]
    StopLossOrderPriceAndDistanceBothMissing,

    /// The Stop Loss Order would cause the associated Trade to be in violation of the FIFO violation safeguard constraints.
    #[serde(rename = "STOP_LOSS_ORDER_WOULD_VIOLATE_FIFO_VIOLATION_SAFEGUARD")]
    StopLossOrderWouldViolateFifoViolationSafeguard,

    /// The Stop Loss Order would be in violation of the risk management Order mutual exclusivity configuration specifying that only one risk management order can be attached to a Trade.
    #[serde(rename = "STOP_LOSS_ORDER_RMO_MUTUAL_EXCLUSIVITY_MUTUALLY_EXCLUSIVE_VIOLATION")]
    StopLossOrderRmoMutualExclusivityMutuallyExclusiveViolation,

    /// The Stop Loss Order would be in violation of the risk management Order mutual exclusivity configuration specifying that if a GSLO is already attached to a Trade, no other risk management Order can be attached to the same Trade.
    #[serde(rename = "STOP_LOSS_ORDER_RMO_MUTUAL_EXCLUSIVITY_GSLO_EXCLUDES_OTHERS_VIOLATION")]
    StopLossOrderRmoMutualExclusivityGsloExcludesOthersViolation,

    /// An attempt to create a pending Order was made with no Stop Loss Order on fill specified, and the Account’s configuration requires that every Trade have an associated Stop Loss Order.
    #[serde(rename = "STOP_LOSS_ON_FILL_REQUIRED_FOR_PENDING_ORDER")]
    StopLossOnFillRequiredForPendingOrder,

    /// An attempt to create a pending Order was made with a Stop Loss Order on fill that was explicitly configured to be guaranteed, however the Account’s configuration does not allow guaranteed Stop Loss Orders.
    #[serde(rename = "STOP_LOSS_ON_FILL_GUARANTEED_NOT_ALLOWED")]
    StopLossOnFillGuaranteedNotAllowed,

    /// An attempt to create a pending Order was made with a Stop Loss Order on fill that was explicitly configured to be not guaranteed, however the Account’s configuration requires guaranteed Stop Loss Orders.
    #[serde(rename = "STOP_LOSS_ON_FILL_GUARANTEED_REQUIRED")]
    StopLossOnFillGuaranteedRequired,

    /// The Stop Loss on fill specified does not provide a prices
    #[serde(rename = "STOP_LOSS_ON_FILL_PRICE_MISSING")]
    StopLossOnFillPriceMissing,

    /// The Stop Loss on fill specifies an invalid prices
    #[serde(rename = "STOP_LOSS_ON_FILL_PRICE_INVALID")]
    StopLossOnFillPriceInvalid,

    /// The Stop Loss on fill specifies a prices with more precision than is allowed by the Order’s instrument
    #[serde(rename = "STOP_LOSS_ON_FILL_PRICE_PRECISION_EXCEEDED")]
    StopLossOnFillPricePrecisionExceeded,

    /// An attempt to create a pending Order was made with the distance between the guaranteed Stop Loss Order on fill’s prices and the pending Order’s prices is less than the Account’s configured minimum guaranteed stop loss distance.
    #[serde(rename = "STOP_LOSS_ON_FILL_GUARANTEED_MINIMUM_DISTANCE_NOT_MET")]
    StopLossOnFillGuaranteedMinimumDistanceNotMet,

    /// An attempt to create a pending Order was made with a guaranteed Stop Loss Order on fill configured, and the Order’s units exceed the Account’s configured guaranteed Stop Loss Order level restriction volume.
    #[serde(rename = "STOP_LOSS_ON_FILL_GUARANTEED_LEVEL_RESTRICTION_EXCEEDED")]
    StopLossOnFillGuaranteedLevelRestrictionExceeded,

    /// The Stop Loss on fill distance is invalid
    #[serde(rename = "STOP_LOSS_ON_FILL_DISTANCE_INVALID")]
    StopLossOnFillDistanceInvalid,

    /// The Stop Loss on fill prices distance exceeds the maximum allowed amount
    #[serde(rename = "STOP_LOSS_ON_FILL_PRICE_DISTANCE_MAXIMUM_EXCEEDED")]
    StopLossOnFillPriceDistanceMaximumExceeded,

    /// The Stop Loss on fill distance contains more precision than is allowed by the instrument
    #[serde(rename = "STOP_LOSS_ON_FILL_DISTANCE_PRECISION_EXCEEDED")]
    StopLossOnFillDistancePrecisionExceeded,

    /// The Stop Loss on fill contains both the prices and distance fields.
    #[serde(rename = "STOP_LOSS_ON_FILL_PRICE_AND_DISTANCE_BOTH_SPECIFIED")]
    StopLossOnFillPriceAndDistanceBothSpecified,

    /// The Stop Loss on fill contains neither the prices nor distance fields.
    #[serde(rename = "STOP_LOSS_ON_FILL_PRICE_AND_DISTANCE_BOTH_MISSING")]
    StopLossOnFillPriceAndDistanceBothMissing,

    /// The Stop Loss on fill specified does not provide a TimeInForce
    #[serde(rename = "STOP_LOSS_ON_FILL_TIME_IN_FORCE_MISSING")]
    StopLossOnFillTimeInForceMissing,

    /// The Stop Loss on fill specifies an invalid TimeInForce
    #[serde(rename = "STOP_LOSS_ON_FILL_TIME_IN_FORCE_INVALID")]
    StopLossOnFillTimeInForceInvalid,

    /// The Stop Loss on fill specifies a GTD TimeInForce but does not provide a GTD timestamp
    #[serde(rename = "STOP_LOSS_ON_FILL_GTD_TIMESTAMP_MISSING")]
    StopLossOnFillGtdTimestampMissing,

    /// The Stop Loss on fill specifies a GTD timestamp that is in the past
    #[serde(rename = "STOP_LOSS_ON_FILL_GTD_TIMESTAMP_IN_PAST")]
    StopLossOnFillGtdTimestampInPast,

    /// The Stop Loss on fill client Order ID specified is invalid
    #[serde(rename = "STOP_LOSS_ON_FILL_CLIENT_ORDER_ID_INVALID")]
    StopLossOnFillClientOrderIDInvalid,

    /// The Stop Loss on fill client Order tag specified is invalid
    #[serde(rename = "STOP_LOSS_ON_FILL_CLIENT_ORDER_TAG_INVALID")]
    StopLossOnFillClientOrderTagInvalid,

    /// The Stop Loss on fill client Order comment specified is invalid
    #[serde(rename = "STOP_LOSS_ON_FILL_CLIENT_ORDER_COMMENT_INVALID")]
    StopLossOnFillClientOrderCommentInvalid,

    /// The Stop Loss on fill specified does not provide a TriggerCondition
    #[serde(rename = "STOP_LOSS_ON_FILL_TRIGGER_CONDITION_MISSING")]
    StopLossOnFillTriggerConditionMissing,

    /// The Stop Loss on fill specifies an invalid TriggerCondition
    #[serde(rename = "STOP_LOSS_ON_FILL_TRIGGER_CONDITION_INVALID")]
    StopLossOnFillTriggerConditionInvalid,

    /// A Guaranteed Stop Loss Order for the specified Trade already exists
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_ALREADY_EXISTS")]
    GuaranteedStopLossOrderAlreadyExists,

    /// An attempt was made to create a Guaranteed Stop Loss Order on a hedged Trade (i.e., there is an existing open Trade in the opposing direction), however the Account’s configuration does not allow Guaranteed Stop Loss Orders for hedged Trades/Positions.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_HEDGING_NOT_ALLOWED")]
    HedgingNotAllowed,

    /// An attempt was made to create a Guaranteed Stop Loss Order, however the distance between the current prices and the trigger prices does not meet the Account’s configured minimum Guaranteed Stop Loss distance.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_MINIMUM_DISTANCE_NOT_MET")]
    MinimumDistanceNotMet,

    /// An attempt was made to cancel a Guaranteed Stop Loss Order when the market is open, however the Account’s configuration requires every Trade have an associated Guaranteed Stop Loss Order.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_NOT_CANCELABLE")]
    NotCancelable,

    /// An attempt was made to cancel a Guaranteed Stop Loss Order when the market is halted, however the Account’s configuration requires every Trade have an associated Guaranteed Stop Loss Order.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_HALTED_NOT_CANCELABLE")]
    HaltedNotCancelable,

    /// An attempt was made to cancel and replace a Guaranteed Stop Loss Order when the market is open, however the Account’s configuration prevents the modification of Guaranteed Stop Loss Orders.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_NOT_REPLACEABLE")]
    NotReplaceable,

    /// An attempt was made to cancel and replace a Guaranteed Stop Loss Order when the market is halted, however the Account’s configuration prevents the modification of Guaranteed Stop Loss Orders.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_HALTED_NOT_REPLACEABLE")]
    HaltedNotReplaceable,

    /// An attempt was made to create a Guaranteed Stop Loss Order, however doing so would exceed the Account’s configured guaranteed StopLoss Order level restriction volume.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_LEVEL_RESTRICTION_VOLUME_EXCEEDED")]
    LevelRestrictionVolumeExceeded,

    /// An attempt was made to create a Guaranteed Stop Loss Order, however doing so would exceed the Account’s configured guaranteed StopLoss Order level restriction prices range.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_LEVEL_RESTRICTION_PRICE_RANGE_EXCEEDED")]
    LevelRestrictionPriceRangeExceeded,

    /// The Guaranteed Stop Loss Order request contains both the prices and distance fields.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_PRICE_AND_DISTANCE_BOTH_SPECIFIED")]
    PriceAndDistanceBothSpecified,

    /// The Guaranteed Stop Loss Order request contains neither the prices nor distance fields.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_PRICE_AND_DISTANCE_BOTH_MISSING")]
    PriceAndDistanceBothMissing,

    /// The Guaranteed Stop Loss Order would cause the associated Trade to be in violation of the FIFO violation safeguard constraints.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_WOULD_VIOLATE_FIFO_VIOLATION_SAFEGUARD")]
    WouldViolateFifoViolationSafeguard,

    /// The Guaranteed Stop Loss Order would be in violation of the risk management Order mutual exclusivity configuration specifying that only one risk management order can be attached to a Trade.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_RMO_MUTUAL_EXCLUSIVITY_MUTUALLY_EXCLUSIVE_VIOLATION")]
    RmoMutualExclusivityMutuallyExclusiveViolation,

    /// The Guaranteed Stop Loss Order would be in violation of the risk management Order mutual exclusivity configuration specifying that if a GSLO is already attached to a Trade, no other risk management Order can be attached to the same Trade.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_RMO_MUTUAL_EXCLUSIVITY_GSLO_EXCLUDES_OTHERS_VIOLATION")]
    RmoMutualExclusivityGsloExcludesOthersViolation,

    /// An attempt to create a pending Order was made with no Guaranteed Stop Loss Order on fill specified, and the Account’s configuration requires that every Trade have an associated Guaranteed Stop Loss Order.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_REQUIRED_FOR_PENDING_ORDER")]
    OnFillRequiredForPendingOrder,

    /// An attempt to create a pending Order was made with a Guaranteed Stop Loss Order on fill that was explicitly configured to be guaranteed, however the Account’s configuration does not allow guaranteed Stop Loss Orders.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_NOT_ALLOWED")]
    OnFillNotAllowed,

    /// An attempt to create a pending Order was made with a Guaranteed Stop Loss Order on fill that was explicitly configured to be not guaranteed, however the Account’s configuration requires Guaranteed Stop Loss Orders.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_REQUIRED")]
    OnFillRequired,

    /// The Guaranteed Stop Loss on fill specified does not provide a prices.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_PRICE_MISSING")]
    OnFillPriceMissing,

    /// The Guaranteed Stop Loss on fill specifies an invalid prices.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_PRICE_INVALID")]
    OnFillPriceInvalid,

    /// The Guaranteed Stop Loss on fill specifies a prices with more precision than is allowed by the Order’s instrument.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_PRICE_PRECISION_EXCEEDED")]
    OnFillPricePrecisionExceeded,

    /// An attempt to create a pending Order was made with the distance between the Guaranteed Stop Loss Order on fill’s prices and the pending Order’s prices is less than the Account’s configured minimum guaranteed stop loss distance.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_MINIMUM_DISTANCE_NOT_MET")]
    OnFillMinimumDistanceNotMet,

    /// Filling the Order would result in the creation of a Guaranteed Stop Loss Order with a trigger number of units that violates the account’s Guaranteed Stop Loss Order level restriction volume.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_LEVEL_RESTRICTION_VOLUME_EXCEEDED")]
    OnFillLevelRestrictionVolumeExceeded,

    /// Filling the Order would result in the creation of a Guaranteed Stop Loss Order with a trigger prices that violates the account’s Guaranteed Stop Loss Order level restriction prices range.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_LEVEL_RESTRICTION_PRICE_RANGE_EXCEEDED")]
    OnFillLevelRestrictionPriceRangeExceeded,

    /// The Guaranteed Stop Loss on fill distance is invalid.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_DISTANCE_INVALID")]
    OnFillDistanceInvalid,

    /// The Guaranteed Stop Loss on fill prices distance exceeds the maximum allowed amount.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_PRICE_DISTANCE_MAXIMUM_EXCEEDED")]
    OnFillPriceDistanceMaximumExceeded,

    /// The Guaranteed Stop Loss on fill distance contains more precision than is allowed by the instrument.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_DISTANCE_PRECISION_EXCEEDED")]
    OnFillDistancePrecisionExceeded,

    /// The Guaranteed Stop Loss on fill contains both the prices and distance fields.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_PRICE_AND_DISTANCE_BOTH_SPECIFIED")]
    OnFillPriceAndDistanceBothSpecified,

    /// The Guaranteed Stop Loss on fill contains neither the prices nor distance fields.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_PRICE_AND_DISTANCE_BOTH_MISSING")]
    OnFillPriceAndDistanceBothMissing,

    /// The Guaranteed Stop Loss on fill specified does not provide a TimeInForce.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_TIME_IN_FORCE_MISSING")]
    OnFillTimeInForceMissing,

    /// The Guaranteed Stop Loss on fill specifies an invalid TimeInForce.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_TIME_IN_FORCE_INVALID")]
    OnFillTimeInForceInvalid,

    /// The Guaranteed Stop Loss on fill specifies a GTD TimeInForce but does not provide a GTD timestamp.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_GTD_TIMESTAMP_MISSING")]
    OnFillGtdTimestampMissing,

    /// The Guaranteed Stop Loss on fill specifies a GTD timestamp that is in the past.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_GTD_TIMESTAMP_IN_PAST")]
    OnFillGtdTimestampInPast,

    /// The Guaranteed Stop Loss on fill client Order ID specified is invalid.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_CLIENT_ORDER_ID_INVALID")]
    OnFillClientOrderIdInvalid,

    /// The Guaranteed Stop Loss on fill client Order tag specified is invalid.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_CLIENT_ORDER_TAG_INVALID")]
    OnFillClientOrderTagInvalid,

    /// The Guaranteed Stop Loss on fill client Order comment specified is invalid.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_CLIENT_ORDER_COMMENT_INVALID")]
    OnFillClientOrderCommentInvalid,

    /// The Guaranteed Stop Loss on fill specified does not provide a TriggerCondition.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_TRIGGER_CONDITION_MISSING")]
    OnFillTriggerConditionMissing,

    /// The Guaranteed Stop Loss on fill specifies an invalid TriggerCondition.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ON_FILL_TRIGGER_CONDITION_INVALID")]
    OnFillTriggerConditionInvalid,

    /// A Trailing Stop Loss Order for the specified Trade already exists.
    #[serde(rename = "TRAILING_STOP_LOSS_ORDER_ALREADY_EXISTS")]
    TrailingOrderAlreadyExists,

    /// The Trailing Stop Loss Order would cause the associated Trade to be in violation of the FIFO violation safeguard constraints.
    #[serde(rename = "TRAILING_STOP_LOSS_ORDER_WOULD_VIOLATE_FIFO_VIOLATION_SAFEGUARD")]
    TrailingWouldViolateFifoViolationSafeguard,

    /// The Trailing Stop Loss Order would be in violation of the risk management Order mutual exclusivity configuration specifying that only one risk management order can be attached to a Trade.
    #[serde(rename = "TRAILING_STOP_LOSS_ORDER_RMO_MUTUAL_EXCLUSIVITY_MUTUALLY_EXCLUSIVE_VIOLATION")]
    TrailingRmoMutualExclusivityMutuallyExclusiveViolation,

    /// The Trailing Stop Loss Order would be in violation of the risk management Order mutual exclusivity configuration specifying that if a GSLO is already attached to a Trade, no other risk management Order can be attached to the same Trade.
    #[serde(rename = "TRAILING_STOP_LOSS_ORDER_RMO_MUTUAL_EXCLUSIVITY_GSLO_EXCLUDES_OTHERS_VIOLATION")]
    TrailingRmoMutualExclusivityGsloExcludesOthersViolation,

    /// The Trailing Stop Loss on fill specified does not provide a distance.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_PRICE_DISTANCE_MISSING")]
    TrailingOnFillDistanceMissing,

    /// The Trailing Stop Loss on fill distance is invalid.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_PRICE_DISTANCE_INVALID")]
    TrailingOnFillDistanceInvalid,

    /// The Trailing Stop Loss on fill distance contains more precision than is allowed by the instrument.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_PRICE_DISTANCE_PRECISION_EXCEEDED")]
    TrailingOnFillDistancePrecisionExceeded,

    /// The Trailing Stop Loss on fill prices distance exceeds the maximum allowed amount.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_PRICE_DISTANCE_MAXIMUM_EXCEEDED")]
    TrailingOnFillPriceDistanceMaximumExceeded,

    /// The Trailing Stop Loss on fill prices distance does not meet the minimum allowed amount.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_PRICE_DISTANCE_MINIMUM_NOT_MET")]
    TrailingOnFillPriceDistanceMinimumNotMet,

    /// The Trailing Stop Loss on fill specified does not provide a TimeInForce.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_TIME_IN_FORCE_MISSING")]
    TrailingOnFillTimeInForceMissing,

    /// The Trailing Stop Loss on fill specifies an invalid TimeInForce.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_TIME_IN_FORCE_INVALID")]
    TrailingOnFillTimeInForceInvalid,

    /// The Trailing Stop Loss on fill TimeInForce is specified as GTD but no GTD timestamp is provided.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_GTD_TIMESTAMP_MISSING")]
    TrailingOnFillGtdTimestampMissing,

    /// The Trailing Stop Loss on fill GTD timestamp is in the past.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_GTD_TIMESTAMP_IN_PAST")]
    TrailingOnFillGtdTimestampInPast,

    /// The Trailing Stop Loss on fill client Order ID specified is invalid.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_CLIENT_ORDER_ID_INVALID")]
    TrailingOnFillClientOrderIdInvalid,

    /// The Trailing Stop Loss on fill client Order tag specified is invalid.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_CLIENT_ORDER_TAG_INVALID")]
    TrailingOnFillClientOrderTagInvalid,

    /// The Trailing Stop Loss on fill client Order comment specified is invalid.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_CLIENT_ORDER_COMMENT_INVALID")]
    TrailingOnFillClientOrderCommentInvalid,

    /// A client attempted to create either a Trailing Stop Loss order or an order with a Trailing Stop Loss On Fill specified, which may not yet be supported.
    #[serde(rename = "TRAILING_STOP_LOSS_ORDERS_NOT_SUPPORTED")]
    TrailingOrdersNotSupported,

    /// The Trailing Stop Loss on fill specified does not provide a TriggerCondition.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_TRIGGER_CONDITION_MISSING")]
    TrailingOnFillTriggerConditionMissing,

    /// The Tailing Stop Loss on fill specifies an invalid TriggerCondition.
    #[serde(rename = "TRAILING_STOP_LOSS_ON_FILL_TRIGGER_CONDITION_INVALID")]
    TrailingOnFillTriggerConditionInvalid,
}

/// TransactionFilter represents a filter that can be used when fetching Transactions.
#[derive(Serialize, Deserialize, Debug)]
pub enum TransactionFilter {
    /// Order-related Transactions. These are the Transactions that create, cancel, fill, or trigger Orders.
    #[serde(rename = "ORDER")]
    Order,

    /// Funding-related Transactions.
    #[serde(rename = "FUNDING")]
    Funding,

    /// Administrative Transactions.
    #[serde(rename = "ADMIN")]
    Admin,

    /// Account Create Transaction.
    #[serde(rename = "CREATE")]
    AccountCreate,

    /// Account Close Transaction.
    #[serde(rename = "CLOSE")]
    AccountClose,

    /// Account Reopen Transaction.
    #[serde(rename = "REOPEN")]
    AccountReopen,

    /// Client Configuration Transaction.
    #[serde(rename = "CLIENT_CONFIGURE")]
    ClientConfigure,

    /// Client Configuration Reject Transaction.
    #[serde(rename = "CLIENT_CONFIGURE_REJECT")]
    ClientConfigureReject,

    /// Transfer Funds Transaction.
    #[serde(rename = "TRANSFER_FUNDS")]
    TransferFunds,

    /// Transfer Funds Reject Transaction.
    #[serde(rename = "TRANSFER_FUNDS_REJECT")]
    TransferFundsReject,

    /// Market Order Transaction.
    #[serde(rename = "MARKET_ORDER")]
    MarketOrder,

    /// Market Order Reject Transaction.
    #[serde(rename = "MARKET_ORDER_REJECT")]
    MarketOrderReject,

    /// Limit Order Transaction.
    #[serde(rename = "LIMIT_ORDER")]
    LimitOrder,

    /// Limit Order Reject Transaction.
    #[serde(rename = "LIMIT_ORDER_REJECT")]
    LimitOrderReject,

    /// Stop Order Transaction.
    #[serde(rename = "STOP_ORDER")]
    StopOrder,

    /// Stop Order Reject Transaction.
    #[serde(rename = "STOP_ORDER_REJECT")]
    StopOrderReject,

    /// Market if Touched Order Transaction.
    #[serde(rename = "MARKET_IF_TOUCHED_ORDER")]
    MarketIfTouchedOrder,

    /// Market if Touched Order Reject Transaction.
    #[serde(rename = "MARKET_IF_TOUCHED_ORDER_REJECT")]
    MarketIfTouchedOrderReject,

    /// Take Profit Order Transaction.
    #[serde(rename = "TAKE_PROFIT_ORDER")]
    TakeProfitOrder,

    /// Take Profit Order Reject Transaction.
    #[serde(rename = "TAKE_PROFIT_ORDER_REJECT")]
    TakeProfitOrderReject,

    /// Stop Loss Order Transaction.
    #[serde(rename = "STOP_LOSS_ORDER")]
    StopLossOrder,

    /// Stop Loss Order Reject Transaction.
    #[serde(rename = "STOP_LOSS_ORDER_REJECT")]
    StopLossOrderReject,

    /// Guaranteed Stop Loss Order Transaction.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER")]
    GuaranteedStopLossOrder,

    /// Guaranteed Stop Loss Order Reject Transaction.
    #[serde(rename = "GUARANTEED_STOP_LOSS_ORDER_REJECT")]
    GuaranteedStopLossOrderReject,

    /// Trailing Stop Loss Order Transaction.
    #[serde(rename = "TRAILING_STOP_LOSS_ORDER")]
    TrailingStopLossOrder,

    /// Trailing Stop Loss Order Reject Transaction.
    #[serde(rename = "TRAILING_STOP_LOSS_ORDER_REJECT")]
    TrailingStopLossOrderReject,

    /// One Cancels All Order Transaction.
    #[serde(rename = "ONE_CANCELS_ALL_ORDER")]
    OneCancelsAllOrder,

    /// One Cancels All Order Reject Transaction.
    #[serde(rename = "ONE_CANCELS_ALL_ORDER_REJECT")]
    OneCancelsAllOrderReject,

    /// One Cancels All Order Trigger Transaction.
    #[serde(rename = "ONE_CANCELS_ALL_ORDER_TRIGGERED")]
    OneCancelsAllOrderTriggered,

    /// Order Fill Transaction.
    #[serde(rename = "ORDER_FILL")]
    OrderFill,

    /// Order Cancel Transaction.
    #[serde(rename = "ORDER_CANCEL")]
    OrderCancel,

    /// Order Cancel Reject Transaction.
    #[serde(rename = "ORDER_CANCEL_REJECT")]
    OrderCancelReject,

    /// Order Client Extensions Modify Transaction.
    #[serde(rename = "ORDER_CLIENT_EXTENSIONS_MODIFY")]
    OrderClientExtensionsModify,

    /// Order Client Extensions Modify Reject Transaction.
    #[serde(rename = "ORDER_CLIENT_EXTENSIONS_MODIFY_REJECT")]
    OrderClientExtensionsModifyReject,

    /// Trade Client Extensions Modify Transaction.
    #[serde(rename = "TRADE_CLIENT_EXTENSIONS_MODIFY")]
    TradeClientExtensionsModify,

    /// Trade Client Extensions Modify Reject Transaction.
    #[serde(rename = "TRADE_CLIENT_EXTENSIONS_MODIFY_REJECT")]
    TradeClientExtensionsModifyReject,

    /// Margin Call Enter Transaction.
    #[serde(rename = "MARGIN_CALL_ENTER")]
    MarginCallEnter,

    /// Margin Call Extend Transaction.
    #[serde(rename = "MARGIN_CALL_EXTEND")]
    MarginCallExtend,

    /// Margin Call Exit Transaction.
    #[serde(rename = "MARGIN_CALL_EXIT")]
    MarginCallExit,

    /// Delayed Trade Closure Transaction.
    #[serde(rename = "DELAYED_TRADE_CLOSURE")]
    DelayedTradeClosure,

    /// Daily Financing Transaction.
    #[serde(rename = "DAILY_FINANCING")]
    DailyFinancing,

    /// Reset Resettable PL Transaction.
    #[serde(rename = "RESET_RESETTABLE_PL")]
    ResetResettablePL,
}

/// TransactionHeartbeat represents a heartbeat object injected into the Transaction stream.
#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionHeartbeat {
    /// The type of the heartbeat (always "HEARTBEAT").
    #[serde(rename = "type")]
    pub type_of: String,

    /// The ID of the most recent Transaction created for the Account.
    #[serde(rename = "lastTransactionID")]
    pub last_transaction_id: TransactionID,

    /// The date/time when the TransactionHeartbeat was created.
    pub time: DateTime,
}

/// The reason that the Market-if-touched Order was initiated.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[allow(dead_code)]
#[allow(unused)]
pub enum MarketIfTouchedOrderReason {
    /// The Market-if-touched Order was initiated at the request of a client.
    #[serde(rename = "CLIENT_ORDER")]
    ClientOrder,

    /// The Market-if-touched Order was initiated as a replacement for an existing Order.
    #[serde(rename = "REPLACEMENT")]
    Replacement,
}


