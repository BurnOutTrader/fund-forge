
# Live Candle Subscriptions
## Time Bars
```rust
fn example() {
    let req = RequestTimeBarUpdate {
        template_id: 200,
        user_msg: vec![],
        symbol: Some("NQ".to_string()),
        exchange: Some(Exchange::CME.to_string()),
        request: Some(Request::Subscribe.into()),
        bar_type: Some(1),
        bar_type_period: Some(5), 
    };
}

pub enum Request {
    Subscribe = 1,
    Unsubscribe = 2,
}

pub enum BarType {
    SecondBar = 1,
    MinuteBar = 2,
    DailyBar = 3,
    WeeklyBar = 4,
}
```
***Outputs***
```
Time Bar (Template ID: 250) from Server: TimeBar 
{ template_id: 250, symbol: Some("NQ"), exchange: Some("CME"), r#type: Some(SecondBar), period: Some("5"), 
marker: Some(1727422825), num_trades: Some(83), volume: Some(86), bid_volume: Some(13), ask_volume: Some(73), open_price: Some(20251.75), 
close_price: Some(20246.75), high_price: Some(20253.25), low_price: Some(20246.25), settlement_price: None, has_settlement_price: None, must_clear_settlement_price: None }
```

## Tick Bars
Doesn't work, might need level 2 data feed
```rust
fn example() {
    pub enum BarType {
        TickBar = 1,
        RangeBar = 2,
        VolumeBar = 3,
    }
    pub enum BarSubType {
        Regular = 1,
        Custom = 2,
    }

    let req = RequestTickBarUpdate {
        template_id: 204,
        user_msg: vec![],
        symbol: Some("NQ".to_string()),
        exchange: Some(Exchange::CME.to_string()),
        request: Some(Request::Subscribe.into()),
        bar_type: Some(1),
        bar_sub_type: Some(1),
        bar_type_specifier: None,
        custom_session_open_ssm: None,
        custom_session_close_ssm: None,
    };
}
```

