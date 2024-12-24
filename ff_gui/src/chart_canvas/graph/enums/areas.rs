#[derive(Clone, PartialEq, Copy)]
pub enum ChartAreas {
    DrawingArea,
    DataArea,
    PriceScale,
    DateScale,
    ControlPanel,
    None,
}