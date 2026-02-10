#[cfg(test)]
mod tests {
    use cudarc::driver::CudaContext;

    #[test]
    fn test_cuda_initialization() {
        println!("Attempting to initialize CUDA device 0...");
        let ctx = CudaContext::new(0);
        match ctx {
            Ok(c) => {
                println!("Successfully initialized CUDA context: {:?}", c.ordinal());
                assert!(true);
            }
            Err(e) => {
                eprintln!("Failed to initialize CUDA device: {:?}", e);
                // Don't panic on CI/machines without GPU - just report
                panic!("CUDA Initialization failed: {:?}", e);
            }
        }
    }
}
