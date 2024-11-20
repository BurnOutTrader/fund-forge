use std::sync::Arc;
use structopt::lazy_static::lazy_static;

lazy_static! {
    pub static ref DATA_BENTO_CLIENT: Arc<DataBentoClient> = Arc::new(DataBentoClient::new());
}

pub struct DataBentoClient {

}

impl DataBentoClient {
    fn new() -> Self {
        Self {
        }
    }

    pub fn shutdown(&self) {

    }
}

