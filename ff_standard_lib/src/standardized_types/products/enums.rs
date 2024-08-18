
/*
pub ProductEnum {

}

impl ProductEnum {
    /// helper method to serialize the Vec<InstrumentEnum> to the array of bytes
    pub fn array_to_bytes(price_data: Vec<ProductEnum>) -> AlignedVec {
        // Create a new serializer
        let mut serializer = AllocSerializer::<1024>::default();

        // Serialize the Vec<QuoteBar>
        serializer.serialize_value(&price_data).unwrap();

        // Get the serialized bytes
        let vec = serializer.into_serializer().into_inner();
        vec
    }
}

impl TradableProduct for ProductEnum {
    fn min_tradeable_amount(&self) -> u64 {
        match self {
            ProductEnum::OandaProduct(instrument) => instrument.min_tradeable_amount(),
        }
    }

    fn max_tradeable_amount(&self) -> u64 {
        match self {
            ProductEnum::OandaProduct(instrument) => instrument.max_tradeable_amount(),
        }
    }

    fn round_trade_size(&self, original_size: u64) -> u64 {
        match self {
            ProductEnum::OandaProduct(instrument) => instrument.round_trade_size(original_size),
        }
    }

    fn margin_rate(&self) -> f64 {
        match self {
            ProductEnum::OandaProduct(instrument) => instrument.margin_rate(),
        }
    }

    fn market_type(&self) -> MarketType {
        match self {
            ProductEnum::OandaProduct(instrument) => instrument.market_type(),
        }
    }

    fn overnight_margin_requirement(&self, quantity: u64, entry_price: f64) -> f64 {
        match self {
            ProductEnum::OandaProduct(instrument) => instrument.overnight_margin_requirement(quantity, entry_price),
        }
    }

    fn day_margin_requirement(&self, quantity: u64, entry_price: f64) -> f64 {
        match self {
            ProductEnum::OandaProduct(instrument) => instrument.day_margin_requirement(quantity, entry_price),
        }
    }

    fn margin_requirement(&self, quantity: u64, entry_price: f64) -> f64 {
        match self {
            ProductEnum::OandaProduct(instrument) => instrument.margin_requirement(quantity, entry_price),
        }
    }
}



impl Display for ProductEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProductEnum::OandaProduct(instrument) => write!(f, "{}", instrument.symbol),
        }
    }
}

impl ViewableProduct for ProductEnum {
    /// Returns the decimal precision for the product.
    ///
    /// This method determines the number of decimal places that should be used when displaying
    /// the product's price or value. The precision is specific to the type of product and is
    /// necessary for correctly formatting its representation in a user interface or report.
    ///
    /// # Returns
    /// A `usize` representing the number of decimal places to use for the product's price or value.
    fn decimal_precision(&self) -> usize {
        match self {
            ProductEnum::OandaProduct(instrument) => instrument.display_precision as usize,
        }
    }
    
    /// Returns a cleaned version of the instrument's symbol.
    ///
    /// This method processes the `ProductEnum` to extract and clean the symbol of the contained `ProductEnum::Variant`.
    /// The cleaning process is intended to adjust the symbol format for consistency or readability, though the
    /// specific transformations are handled by the `clean_symbol()` function, which is not detailed here.
    ///
    /// # Returns
    /// A `String` representing the cleaned symbol of the instrument.
    fn symbol(&self) -> String {
        match self {
            ProductEnum::OandaProduct(instrument) => fund_forge_formatted_symbol(&instrument.symbol),
        }
    }

    /// Returns the vendor-specific symbol format for the instrument.
    ///
    /// This method provides the symbol of the instrument as it is recognized by the vendor, which in this case is Oanda.
    /// This can be useful for applications that need to communicate with external systems or APIs that use the vendor's
    /// specific naming conventions for instruments.
    ///
    /// # Returns
    /// A `String` representing the instrument's name in the vendor-specific format.
    fn symbol_vendor_format(&self) -> String {
        match self {
            ProductEnum::OandaProduct(instrument) => instrument.instrument_name.clone(),
        }
    }
}

/// helper method to deserialize the Vec<InstrumentEnum> from the array of bytes
pub fn products_from_array_bytes(data: &Vec<u8>) -> Result<Vec<ProductEnum>, core::fmt::Error> {
    let archived_price_data = match rkyv::check_archived_root::<Vec<ProductEnum>>(&data[..]){
        Ok(price_data) => price_data,
        Err(e) => {
            format!("Failed to deserialize InstrumentEnum: {}", e);
            return Err(core::fmt::Error);
        },
    };

    // Assuming you want to work with the archived data directly, or you can deserialize it further
    Ok(archived_price_data.deserialize(&mut rkyv::Infallible).unwrap())
}*/

