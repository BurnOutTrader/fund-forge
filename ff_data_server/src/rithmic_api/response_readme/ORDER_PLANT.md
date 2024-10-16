
## Accounts List
```rust
fn example() {
    let accounts = RequestAccountList {
        template_id: 302,
        user_msg: vec![],
        fcm_id: None,
        ib_id: None,
        user_type: Some(UserType::Trader.into())
    };
}
``` 
***Accounts Response***
Account List Response (Template ID: 303) from Server: ResponseAccountList { template_id: 303, user_msg: [], rq_handler_rp_code: ["0"], rp_code: [], fcm_id: Some("TopstepTrader"), ib_id: Some("TopstepTrader"), account_id: Some("S1Sep246906077"), account_name: Some("S1Sep246906077"), account_currency: Some("USD"), account_auto_liquidate: Some("enabled"), auto_liq_threshold_current_value: None }

## Account RMS Info
```rust
fn example() {
    let rms_req = RequestAccountRmsInfo {
        template_id: 304,
        user_msg: vec![],
        fcm_id: None,
        ib_id: None,
        user_type: Some(UserType::Trader.into()),
    };
}
```
***Account RMS Response***
Response Account Rms Info (Template ID: 305) from Server: ResponseAccountRmsInfo { template_id: 305, user_msg: [], rq_handler_rp_code: ["0"], 
rp_code: [], presence_bits: Some(1023), fcm_id: Some("TopstepTrader"), ib_id: Some("TopstepTrader"), account_id: Some("S1Sep246906077"), 
currency: Some("USD"), status: Some("active"), algorithm: Some("Max Loss"), auto_liquidate_criteria: Some("Multiple Simultaneous Criteria"), 
auto_liquidate: Some(Enabled), disable_on_auto_liquidate: None, auto_liquidate_threshold: Some(1000.0), auto_liquidate_max_min_account_balance: 
Some(50000.0), loss_limit: Some(1000.0), min_account_balance: Some(0.0), min_margin_balance: Some(0.0), default_commission: None, 
buy_limit: Some(5), max_order_quantity: Some(10), sell_limit: Some(5), check_min_account_balance: Some(false) }

## Login Info 
This is useful to get fcm_id and ib_id for other request types and to create login template
```rust
RequestLoginInfo {
    template_id: 300 ,
    user_msg: vec![],
};
```
***Account RMS Response***
Response Login Info (Template ID: 303) from Server: ResponseLoginInfo { template_id: 301, user_msg: [], rp_code: ["0"], fcm_id: Some("xxx"), ib_id: Some("xxxx"), 
first_name: Some("xx"), last_name: Some("xxx"), user_type: Some(Trader) }

    
    
***Order Notification***
Rithmic Order Notification (Template ID: 351) from Server: RithmicOrderNotification { template_id: 351, user_tag: None, notify_type: Some(OrderRcvdFromClnt), 
is_snapshot: None, status: Some("Order received from client"), basket_id: Some("233651480"), original_basket_id: None, linked_basket_ids: None, 
fcm_id: Some("TopstepTrader"), ib_id: Some("TopstepTrader"), user_id: Some("kevtaz"), account_id: Some("S1Sep246906077"), symbol: Some("M6AZ4"), 
exchange: Some("CME"), trade_exchange: Some("CME"), trade_route: Some("simulator"), exchange_order_id: None, instrument_type: None, 
completion_reason: None, quantity: Some(2), quan_release_pending: None, price: None, trigger_price: None, transaction_type: Some(Sell), 
duration: Some(Day), price_type: Some(Market), orig_price_type: Some(Market), manual_or_auto: Some(Manual), bracket_type: None, 
avg_fill_price: None, total_fill_size: None, total_unfilled_size: None, trail_by_ticks: None, trail_by_price_id: None, sequence_number: None, 
orig_sequence_number: None, cor_sequence_number: None, currency: None, country_code: None, text: None, report_text: None, remarks: None, 
window_name: Some("Quote Board, Sell Button, Confirm "), originator_window_name: None, cancel_at_ssboe: None, cancel_at_usecs: None, cancel_after_secs: None, 
ssboe: Some(1729085413), usecs: Some(477767) }



