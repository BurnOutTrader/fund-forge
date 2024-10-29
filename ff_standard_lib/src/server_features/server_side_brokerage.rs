use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crate::messages::data_server_messaging::DataServerResponse;
use crate::standardized_types::enums::StrategyMode;
use crate::standardized_types::new_types::Volume;
use crate::standardized_types::orders::{Order, OrderId, OrderUpdateEvent, OrderUpdateType};
use crate::standardized_types::subscriptions::SymbolName;
use crate::standardized_types::accounts::{Account, AccountId};
use crate::StreamName;

/// The trait allows the server to implement the vendor specific methods for the DataVendor enum without the client needing to implement them.
#[async_trait]
pub trait BrokerApiResponse: Sync + Send {
    /// return `DataServerResponse::Symbols` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    ///
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    ///
    /// You can return a hard coded list here for most brokers, equities we might want to get dynamically from the brokerage.
    async fn symbol_names_response(
        &self,
        mode: StrategyMode,
        time: Option<DateTime<Utc>>,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::AccountInfo` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    ///
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    async fn account_info_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        account_id: AccountId,
        callback_id: u64
    ) -> DataServerResponse;

    async fn paper_account_init(
        &self,
        account_id: AccountId,
        callback_id: u64
    ) -> DataServerResponse;

    /// return` DataServerResponse::SymbolInfo` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    ///
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    ///
    /// You can return a hard coded list here for most brokers, equities we might want to get dynamically from the brokerage.
    async fn symbol_info_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::IntraDayMarginRequired` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    ///
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    ///
    /// Margin required for x units of the symbol, the mode is passed in
    /// We can return hard coded values for backtesting and live values for live or live paper
    /// You can return a hard coded list here for most brokers, since margin requirements can change we might want to handle historical and live differently.
    /// equities we might want to get dynamically from the brokerage, or use a calculation based on current market price
    async fn intraday_margin_required_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse;


    /// return `DataServerResponse::OvernightMarginRequired` or `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    ///
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    ///
    /// Margin required for x units of the symbol, the mode is passed in
    /// We can return hard coded values for backtesting and live values for live or live paper
    /// You can return a hard coded list here for most brokers, since margin requirements can change we might want to handle historical and live differently.
    /// equities we might want to get dynamically from the brokerage, or use a calculation based on current market price
    async fn overnight_margin_required_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        symbol_name: SymbolName,
        quantity: Volume,
        callback_id: u64
    ) -> DataServerResponse;

    /// return `DataServerResponse::Accounts or DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    ///
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    async fn accounts_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        callback_id: u64
    ) -> DataServerResponse;

    /// This command doesn't require a response,
    /// it is sent when a connection is dropped so that we can remove any items associated with the stream
    /// (strategy that is connected to this port)
    async fn logout_command(
        &self,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
    );

    /// Returns a `DataServerResponse::CommissionInfo` object if symbol is found, else returns a `DataServerResponse::Error{error: FundForgeError, callback_id: u64}`
    ///
    /// `FundForgeError::ServerSideErrorDebug` or `FundForgeError::ClientSideErrorDebug` depending on who caused this problem.
    ///
    /// You can return a hard coded list here for most brokers or we can ask the broker dynamically,
    /// we could also serialize a static csv and parse it to a static map on start up, this would allow us to manually edit changes to commissions.
    async fn commission_info_response(
        &self,
        mode: StrategyMode,
        // The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
        // it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
        stream_name: StreamName,
        symbol_name: SymbolName,
        callback_id: u64
    ) -> DataServerResponse;

    /// Executes a live market order based on the given strategy mode and order.
    ///
    /// # Arguments
    /// * `stream_name: StreamName` - The u16 port number for the connected strategy, this is useful for callbacks, to be used as part of your callback id system if you need one.
    /// * `mode: StrategyMode` - The strategy mode in which the order is executed (e.g., Live, Paper, Backtest).
    /// * `order: Order` - The order object containing all the order details (e.g., brokerage, account ID, quantity).
    ///
    /// # Returns
    /// * `Ok(())` - If the order passes validation and is successfully handled.
    /// * `Err(OrderUpdateEvent::OrderRejected)` - If the order is invalid, returns an order rejection event.
    ///
    /// return `Ok(())` if your api has successfully passed the order on to the broker. \
    /// return `Err(OrderUpdateEvent::Rejected())` If the api has rejected the order before passing it to the broker. \
    /// This will allow you to handle validation logic on the fund forge server before forwarding the order to the brokerage. \
    /// The brokerage confirmation will arrive later, and we will need to parse that future result and return another OrderUpdateEvent, but we do not do that here.
    ///
    /// This result is only for the api clients own validation logic before we forward an order to the broker for acceptance. \
    /// For example, The Rithmic client will check if the order is going to put us over our max position size, if it is then it will not forward to the broker and will instead return the `Err()`.
    ///
    /// If the order is not going to put the account over its max position size then the client will forward the order and return `Ok(())`
    /// This helps to stop from breaking rules in online prop firm accounts commonly used with rithmic.
    /// # Example
    /// ```rust
    /// use chrono::Utc;
    /// use ff_standard_lib::standardized_types::accounts::Account;
    /// use ff_standard_lib::standardized_types::enums::StrategyMode;
    /// use ff_standard_lib::standardized_types::orders::{Order, OrderUpdateEvent};
    ///
    /// struct ExampleApi;
    /// impl ExampleApi {
    ///     async fn live_market_order(&self, mode: StrategyMode, order: Order) -> Result<(), OrderUpdateEvent> {
    ///         // Validate the order before proceeding. Here the client checks that we will not be over the accounts max position size
    ///         match self.is_valid_order(&order) {
    ///             // If the order is invalid, return an `OrderRejected` event with details.
    ///             Err(e) => {
    ///                 Err(OrderUpdateEvent::OrderRejected {
    ///                     account: Account { brokerage: Brokerage::Test,account_id: "".to_string()},
    ///                     symbol_name: "".to_string(),
    ///                     symbol_code: "".to_string(),order_id: order.id,  // The order Id of the order.
    ///                     reason: e,  // Reason for rejection, provided by validation.
    ///                     tag: order.tag,  // we use the order tag so the strategy can identify the order responsible tag.
    ///                     time: Utc::now().to_string(),  // Utc Time String of the rejection event.
    ///                 })
    ///             }
    ///             // If the order is valid, continue processing (logic to be added here).
    ///             Ok(_) => {
    ///                 // Example: Execute the order or send it to the brokerage system.
    ///                 Ok(())
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    async fn live_market_order(
        &self,
        stream_name: StreamName,
        mode: StrategyMode,
        order: Order,
    ) -> Result<(), OrderUpdateEvent>;

    /// The Err order event is only returned if there is some error that occured in our logic,
    /// a successful update event should come from the broker side
    async fn live_enter_long(
        &self,
        stream_name: StreamName,
        mode: StrategyMode,
        order: Order,
    ) -> Result<(), OrderUpdateEvent>;

    /// The Err order event is only returned if there is some error that occured in our logic,
    /// a successful update event should come from the broker side
    async fn live_enter_short(
        &self,
        stream_name: StreamName,
        mode: StrategyMode,
        order: Order,
    ) -> Result<(), OrderUpdateEvent>;

    /// The Err order event is only returned if there is some error that occured in our logic,
    /// a successful update event should come from the broker side
    async fn live_exit_short(
        &self,
        stream_name: StreamName,
        mode: StrategyMode,
        order: Order,
    ) -> Result<(), OrderUpdateEvent>;

    /// The Err order event is only returned if there is some error that occured in our logic,
    /// a successful update event should come from the broker side
    async fn live_exit_long(
        &self,
        stream_name: StreamName,
        mode: StrategyMode,
        order: Order,
    ) -> Result<(), OrderUpdateEvent>;

    // Handle other order types, Limit, Stop Limit, Stop Market, MIT etc
    /// The Err order event is only returned if there is some error that occured in our logic,
    /// a successful update event should come from the broker side
    async fn other_orders(
        &self,
        stream_name: StreamName,
        mode: StrategyMode,
        order: Order,
    ) -> Result<(), OrderUpdateEvent>;

    /// cancel all pending orders on an account
    async fn cancel_orders_on_account(
        &self,
        account: Account,
    );

    ///cancel a specific order
    async fn cancel_order(
        &self,
        account: Account,
        order_id: OrderId,
    );

    ///flatten the entire account including cancelling any orders
    async fn flatten_all_for(
        &self,
        account: Account
    );

    /// update the existing order based on update type, the order event is only returned if there is some error that occured in our logic,
    /// a successful update event should come from the broker side
    async fn update_order(
        &self,
        account: Account,
        order_id: OrderId,
        update: OrderUpdateType
    ) -> Result<(), OrderUpdateEvent>;
}