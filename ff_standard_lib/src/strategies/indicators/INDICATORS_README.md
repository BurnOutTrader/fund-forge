## Using Indicators
The indicator handler uses dynamic dispatch, there was no noticeable runtime cost since looking up the correct type in a Vtable is negligible compared to running the actual indicator logic.

Any object that implements the [Indicators](indicators_trait.rs) trait can be used as an indicator.

It is easy to create and add indicators or custom candlestick types. Below we subscribe to an ATR indicator using Heikin Ashi candles.

Indicators can also be set to keep a history, so you can call the last .index(0) objects without having to manually retain the history.

Indicators will warm themselves up on creation if the strategy is already warmed up, so we can subscribe and unsubscribe at any time.
See [Indicators readme](ff_standard_lib/src/strategies/indicators/INDICATORS_README.md)
```rust
fn example() {
  // Here we create a 5 period ATR using a Heikin Ashi data subscription, and we specify to retain the last 100 bars in memory.
  let heikin_atr_5: Box<dyn Indicators> = 
    AverageTrueRange::new(
      IndicatorName::from("heikin_atr_5"),
      DataSubscription::new_custom(
        SymbolName::from("EUR-USD"),
        DataVendor::Test,
        Resolution::Seconds(5),
        MarketType::Forex,
        CandleType::HeikinAshi,
      ),
      100, //retain 100 last values
      5, // atr period
      Some(Color::new(50,50,50)) //plot color rgb for charting 
    ).await;
  
  // auto subscribe will subscribe the strategy to the indicators required data feed if it is not already, 
  // if this is false and you don't have the subscription, the strategy will panic instead.
  // if true then the new data subscription will also show up in the strategy event loop
  let auto_subscribe: bool = false;
  
  //subscribe the strategy to auto manage the indicator
  strategy.subscribe_indicator(heikin_atr_5, auto_subscribe).await;
}
```

## Creating Your Own Indicators
I have chosen to use enums and matching statements over dynamic dispatch for increased performance at the cost of simply completeing a matching statement.

*I will add another enum type and trait for multi symbol indicators in the future.*

### Step 1
The `new()` function for your indicator should return a Box<Self>

### Step 2
2. Create a new Indicator object that implements the [Indicators](indicators_trait.rs) trait
also see [AverageTrueRange](built_in/average_true_range.rs) for a working example.
```rust
pub struct YOUR_NEW_VARIANT {
    history: RollingWindow<IndicatorValues>, //keep a history of indicator values

    //if your indicator needs to keep some data just use this
    base_data_history: RollingWindow<BaseDataEnum>, 
    
    // the subscriptions or subscription your indicator uses for primary data
    subscriptions: DataSubsciption
}

impl YOUR_NEW_VARIANT {
    // you can put non trait logic here
}

impl Indicators for YOUR_NEW_VARIANT {
    fn name(&self) -> IndicatorName {
        self.name.clone()
    }

    fn history_to_retain(&self) -> usize {
        self.history.number.clone() as usize
    }

    fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<IndicatorValues> {
        todo!()
    }

    fn subscription(&self) -> DataSubscription {
        self.subscription.clone()
    }

    fn reset(&mut self) {
        self.history.clear();
        self.base_data_history.clear();
    }

    fn index(&self, index: usize) -> Option<IndicatorValues> {
        if !self.is_ready {
            return None;
        }
        self.history.get(index).cloned()
    }

    fn current(&self) -> Option<IndicatorValues> {
        if !self.is_ready {
            return None;
        }
        self.history.last().cloned()
    }

    fn plots(&self) -> RollingWindow<IndicatorValues> {
        self.history.clone()
    }

    fn is_ready(&self) -> bool {
        self.is_ready
    }

    fn history(&self) -> RollingWindow<IndicatorValues> {
        self.history.clone()
    }

    fn data_required_warmup(&self) -> u64 {
        self.history.len() as u64 + self.period
    }
}
```

### Step 2
Your indicator can now be auto managed by using strategy.subscribe_indicator(), including auto warm up and other automatic features.
You don't need to do anything else other than test it.