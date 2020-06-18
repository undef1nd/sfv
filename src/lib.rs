pub mod parser;
pub mod serializer;
mod utils;

type Res<R> = Result<R, &'static str>;
