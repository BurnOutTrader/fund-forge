# Creating Your Own Indicators
I have chosen to use enums and matching statements over dynamic dispatch for increased performance at the cost of simply completeing a matching statement.

*I will add another enum type and trait for multi symbol indicators in the future.*

## Step 1
1. Create a new Indicator object that implements the [Indicators](indicators_trait.rs) trait
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
## Step 2
Create a new [IndicatorEnum Variant](indicator_enum.rs)
```rust
pub enum IndicatorEnum {
    Custom(Box<dyn Indicators + Send + Sync>), //if we use this then we cant use rkyv serialization
    AverageTrueRange(AverageTrueRange),
    YOUR_NEW_VARIANT(YOUR_NEW_VARIANT)
}

```

## Step 3
Complete the matching statements for your new variant in indicator enum impl.
This is easy since you implement the exact same trait as the IndicatorEnum.
```rust
impl Indicators for IndicatorEnum {
    fn name(&self) -> IndicatorName {
        match self {
            IndicatorEnum::AverageTrueRange(atr) => atr.name(),
            IndicatorEnum::Custom(indicator) => indicator.name(),
            IndicatorEnum::YOUR_NEW_VARIANT(new_indicator) => new_indicator.name()
        }
    }
}
```

## Step 4
Your indicator can now be auto managed by using strategy.subscribe_indicator(), including auto warm up and other automatic features.
You don't need to do anything else other than test it.