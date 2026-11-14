use crate::error::HypercError;

use inkwell::{
    module::Module,
    OptimizationLevel,
};

pub fn run_jit(module: &Module) -> Result<i32, HypercError> {
    let execution_engine = module
        .create_jit_execution_engine(OptimizationLevel::None)
        .map_err(|e| HypercError::BuildError { message: e.to_string() })?;

    type MainFn = unsafe extern "C" fn() -> i32;

    let main_fn = unsafe {
        execution_engine.get_function::<MainFn>("main")
            .map_err(|e| HypercError::BuildError {
                message: format!("Unable to get the main function: {e}")
            }
        )?
    };

    let code = unsafe { main_fn.call() };

    Ok(code)
}
