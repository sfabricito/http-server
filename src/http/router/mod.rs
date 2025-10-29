mod router;
mod jobs;
mod cpu_bound;
mod io

pub use router::{QueryParam, SimpleHandler, build_routes};
pub use jobs::register
pub use cpu_bound::register 
pub use io_bound::register;