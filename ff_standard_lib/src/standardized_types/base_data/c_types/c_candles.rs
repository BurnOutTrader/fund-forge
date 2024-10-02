#[repr(C)]
pub struct CandleC {
    symbol: *const c_char,  // Use C strings for compatibility
    high: f64,
    low: f64,
    open: f64,
    close: f64,
    volume: f64,
    range: f64,
    time: *const c_char,
    is_closed: bool,
    data_vendor: i32,  // Assuming DataVendor is also represented as an integer
    resolution: i32,   // Assuming Resolution is also represented as an integer
}

impl From<Candle> for CandleC {
    fn from(candle: Candle) -> Self {
        CandleC {
            symbol: CString::new(candle.symbol).unwrap().into_raw(),
            high: candle.high,
            low: candle.low,
            open: candle.open,
            close: candle.close,
            volume: candle.volume,
            range: candle.range,
            time: CString::new(candle.time).unwrap().into_raw(),
            is_closed: candle.is_closed,
            data_vendor: candle.data_vendor as i32,
            resolution: candle.resolution as i32,
        }
    }
}

// Remember to provide get to free CString allocations to prevent memory leaks!

#[no_mangle]
pub extern "C" fn create_candle(symbol: *const c_char, high: f64, low: f64, open: f64, close: f64, volume: f64, range: f64, time: *const c_char, is_closed: bool, data_vendor: i32, resolution: i32) -> *mut CandleC {
    let candle = Candle {
        symbol: unsafe { CStr::from_ptr(symbol).to_string_lossy().into_owned() },
        high,
        low,
        open,
        close,
        volume,
        range,
        time: unsafe { CStr::from_ptr(time).to_string_lossy().into_owned() },
        is_closed,
        data_vendor: unsafe { transmute(data_vendor) },  // DataVendor should be safely transmutable
        resolution: unsafe { transmute(resolution) },    // Resolution should be safely transmutable
    };
    Box::into_raw(Box::new(CandleC::from(candle)))
}

#[no_mangle]
pub extern "C" fn free_candle(ptr: *mut CandleC) {
    unsafe {
        let _ = Box::from_raw(ptr); // Automatically frees memory when it goes out of scope
    }
}