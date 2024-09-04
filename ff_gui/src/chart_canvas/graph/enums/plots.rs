
#[derive(Debug, PartialEq, Clone)]
pub enum Shape {
    Circle,
    Square,
    TriangleUp,
    TriangleDown,
    TriangleLeft,
    TriangleRight,
}

#[derive(Debug, PartialEq, Clone)]
pub enum PlotStyle {
    Line,
    Bar,
    Shape(Shape),
}

