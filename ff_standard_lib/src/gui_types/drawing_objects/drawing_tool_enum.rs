use std::fmt::Error;

use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};

use crate::gui_types::drawing_objects::lines::{HorizontalLine, VerticleLine};
use crate::standardized_types::subscriptions::DataSubscription;

//ToDo make drawing tool trait so we can implement depending on if strategy is using or if gui is using... or convert from strategy to gui type tool.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum DrawingTool {
    HorizontalLines(HorizontalLine),
    VerticleLines(VerticleLine),
}

impl DrawingTool {
    pub fn subscription(&self) -> &DataSubscription {
        match self {
            DrawingTool::HorizontalLines(object) => &object.subscription,
            DrawingTool::VerticleLines(object) => &object.subscription,
        }
    }

    pub fn from_array_bytes(data: &Vec<u8>) -> Result<Vec<DrawingTool>, Error> {
        let drawing_tools = match rkyv::check_archived_root::<Vec<DrawingTool>>(&data[..]) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to deserialize tools: {}", e);
                return Err(Error);
            }
        };

        // Assuming you want to work with the archived data directly, or you can deserialize it further
        Ok(drawing_tools.deserialize(&mut rkyv::Infallible).unwrap())
    }

    pub fn is_ready(&self) -> bool {
        match self {
            DrawingTool::VerticleLines(object) => object.is_ready,
            DrawingTool::HorizontalLines(object) => object.is_ready,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            DrawingTool::HorizontalLines(_) => "H Line".to_string(),
            DrawingTool::VerticleLines(_) => "V Line".to_string(),
        }
    }

    pub fn id(&self) -> String {
        match self {
            DrawingTool::HorizontalLines(object) => object.id.clone(),
            DrawingTool::VerticleLines(object) => object.id.clone(),
        }
    }
}
