use super::*;
use crate::runtime as native;

impl CodeGen {
    pub fn run_tests(&mut self, filter: Option<&str>, verbose: bool) -> Result<i64> {
        let mut total = 0i64;
        let mut passed = 0i64;
        let mut failed = 0i64;

        for &init_id in &self.package_inits {
            let init_ptr = self.module.get_finalized_function(init_id);
            let init_fn: fn() = unsafe { std::mem::transmute(init_ptr) };
            init_fn();
        }

        for test in &self.tests {
            if let Some(pattern) = filter {
                if !test.display_name.contains(pattern) && !test.stable_name.contains(pattern) {
                    continue;
                }
            }

            total += 1;
            let name_bytes = test.display_name.as_bytes();
            native::test_api::breom_test_begin(name_bytes.as_ptr(), name_bytes.len() as u64);

            let func_id = *self
                .functions
                .get(&test.function_name)
                .ok_or_else(|| anyhow!("Missing compiled test function: {}", test.function_name))?;
            let func_ptr = self.module.get_finalized_function(func_id);
            let test_fn: fn() = unsafe { std::mem::transmute(func_ptr) };
            test_fn();

            let fail_count = native::test_api::breom_test_end();
            if fail_count == 0 {
                passed += 1;
                if verbose {
                    println!("[PASS] {}", test.display_name);
                }
            } else {
                failed += 1;
                println!("[FAIL] {}", test.display_name);
            }
        }

        if total == 0 {
            println!("No tests found");
            return Ok(3);
        }

        println!("\n{} passed, {} failed, {} total", passed, failed, total);
        Ok(if failed == 0 { 0 } else { 1 })
    }

    pub fn finalize(&mut self) -> Result<()> {
        self.module
            .finalize_definitions()
            .map_err(|e| anyhow!("Failed to finalize: {}", e))?;
        Ok(())
    }

    pub fn run_main(&mut self) -> Result<i64> {
        for &init_id in &self.package_inits {
            let init_ptr = self.module.get_finalized_function(init_id);
            let init_fn: fn() = unsafe { std::mem::transmute(init_ptr) };
            init_fn();
        }

        let func_id = *self
            .functions
            .get("main")
            .ok_or_else(|| anyhow!("No main function found"))?;

        let func_ptr = self.module.get_finalized_function(func_id);

        if self.main_returns_int {
            if self.main_throws {
                let main_fn: fn() -> (i64, i64) = unsafe { std::mem::transmute(func_ptr) };
                let (err, val) = main_fn();
                Ok(if err != 0 { err } else { val })
            } else {
                let main_fn: fn() -> i64 = unsafe { std::mem::transmute(func_ptr) };
                Ok(main_fn())
            }
        } else {
            let main_fn: fn() = unsafe { std::mem::transmute(func_ptr) };
            main_fn();
            Ok(0)
        }
    }
}
