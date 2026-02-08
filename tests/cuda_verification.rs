#[cfg(test)]
mod tests {
    use cudarc::driver::CudaDevice;

    #[test]
    fn test_cuda_initialization() {
        println!("Attempting to initialize CUDA device 0...");
        let dev = CudaDevice::new(0);
        match dev {
            Ok(d) => {
                println!("Successfully initialized CUDA device: {:?}", d.ordinal());
                assert!(true);
            },
            Err(e) => {
                eprintln!("Failed to initialize CUDA device: {:?}", e);
                // We want to see the error, but maybe not fail the test suite if it's just missing hardware on a CI node
                // But for this user loop, we want to know.
                panic!("CUDA Initialization failed: {:?}", e);
            }
        }
    }
}
