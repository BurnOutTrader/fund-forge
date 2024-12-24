
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
pub enum PlotEnum {
    Line,
    Bar,
    Shape(Shape),
}

