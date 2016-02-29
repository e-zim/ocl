// Timed kernel and buffer tests / benchmarks.
//
// Manipulate the consts below to fiddle with parameters. To create longer 
// running tests, increase `DATASET_SIZE`, and the `*_ITERS` consts.
// The other consts can be anything at all

#![feature(time2)]
extern crate ocl;
use std::time::Instant;

use ocl::{ProQue, Buffer, EventList};


const DATASET_SIZE: usize = 10000;

const KERNEL_RUN_ITERS: i32 = 800;
const BUFFER_READ_ITERS: i32 = 20;
// const KERNEL_RUN_ITERS: i32 = 100;
// const BUFFER_READ_ITERS: i32 = 100;
const KERNEL_AND_BUFFER_ITERS: i32 = 1000;

const SCALAR: f32 = 1.0;
const INIT_VAL_RANGE: (f32, f32) = (100.0, 200.0);

const PRINT_SOME_RESULTS: bool = true;
const RESULTS_TO_PRINT: usize = 5;


fn main() {
    // Define a kernel:
    let src = r#"
        __kernel void add(
                    __global float const* const source, 
                    __private float scalar,
                    __global float* const result) 
        {
            uint idx = get_global_id(0);
            result[idx] = source[idx] + scalar;
        }
    "#;

    // Set our work dimensions / data set size to something arbitrary:
    let dims = [DATASET_SIZE];

    // Create an all-in-one context, program, and command queue:
    let ocl_pq = ProQue::builder().src(src).build().unwrap();

    // Create init and result buffers:
    let buffer_init: Buffer<f32> = Buffer::with_vec_scrambled(
         INIT_VAL_RANGE, &dims, &ocl_pq.queue());
    let mut buffer_result: Buffer<f32> = Buffer::with_vec(&dims, &ocl_pq.queue());

    // Create a kernel with arguments matching those in the kernel:
    let mut kern = ocl_pq.create_kernel_with_dims("add", dims.clone())
        .arg_buf_named("source", Some(&buffer_init))
        .arg_scl(SCALAR)
        .arg_buf(&buffer_result);


    // ##################################################
    // ##################### KERNEL #####################
    // ##################################################

    print!("\n");
    println!("Enqueuing {} kernel runs... ", KERNEL_RUN_ITERS);

    // Start kernel timer
    let kern_start = Instant::now();

    // Enqueue kernel the first time:
    kern.enqueue();

    // Set kernel source buffer to the same as result:
    kern.set_arg_buf_named("source", Some(&buffer_result)).unwrap();

    // Enqueue kernel for additional iterations:
    for _ in 0..(KERNEL_RUN_ITERS - 1) {
        kern.enqueue();
    }

    // Wait for all kernels to run:
    ocl_pq.queue().finish();
    
    // Print elapsed time for kernels:
    print_elapsed("total elapsed", kern_start);

    // ##################################################
    // ##################### BUFFER #####################
    // ##################################################

    print!("\n");
    println!("Enqueuing {} buffer reads... ", BUFFER_READ_ITERS);

    // Start kernel timer
    let buffer_start = Instant::now();

    // Read results from the device into buffer's local vector:
    for _ in 0..BUFFER_READ_ITERS {
        buffer_result.fill_vec();
    }

    print_elapsed("queue unfinished", buffer_start);
    ocl_pq.queue().finish();    
    print_elapsed("queue finished", buffer_start);

    verify_results(&buffer_init, &buffer_result, KERNEL_RUN_ITERS);

    // ##################################################
    // ########### KERNEL & BUFFER BLOCKING #############
    // ##################################################

    print!("\n");
    println!("Enqueuing {} blocking kernel buffer sequences... ", KERNEL_AND_BUFFER_ITERS);

    let kern_buf_start = Instant::now();

    for _ in 0..(KERNEL_AND_BUFFER_ITERS) {
        kern.enqueue();
        buffer_result.fill_vec();
    }

    print_elapsed("queue unfinished", kern_buf_start);
    ocl_pq.queue().finish();    
    print_elapsed("queue finished", kern_buf_start);

    verify_results(&buffer_init, &buffer_result, KERNEL_AND_BUFFER_ITERS + KERNEL_RUN_ITERS);

    // ##################################################
    // ######### KERNEL & BUFFER NON-BLOCKING ###########
    // ##################################################

    print!("\n");
    println!("Enqueuing {} non-blocking kernel buffer sequences... ", KERNEL_AND_BUFFER_ITERS);

    let kern_buf_start = Instant::now();

    let mut kern_events = EventList::new();
    let mut buf_events = EventList::new();


    for i in 0..KERNEL_AND_BUFFER_ITERS {
        // if i < 20 { println!("0.0 [{}] ", i); }

        kern.enqueue_with(None, Some(&buf_events), Some(&mut kern_events)).unwrap();
        // kern.enqueue_with(None, None, Some(&mut kern_events));
        // kern.enqueue_with(None, Some(&buf_events), None);
        // kern.enqueue();

        // if i < 20 { println!("0.1 [{}] ", i); }

        unsafe { buffer_result.enqueue_fill_vec(false, Some(&kern_events), Some(&mut buf_events)).unwrap(); }
        // unsafe { buffer_result.enqueue_fill_vec(None, Some(&mut buf_events)).unwrap(); }
        // unsafe { buffer_result.enqueue_fill_vec(Some(&kern_events), None).unwrap(); }
        // unsafe { buffer_result.enqueue_fill_vec(None, None).unwrap(); }

        // if i < 20 { println!("0.2 [{}] ", i); }
        if i < 20 { 
            // println!("kern_events: {:?}, buf_events: {:?} ", kern_events, buf_events); 
            // print!("\n");
        }
    }

    print_elapsed("queue unfinished", kern_buf_start);
    ocl_pq.queue().finish();    
    print_elapsed("queue finished", kern_buf_start);

    kern_events.wait();
    buf_events.wait();

    verify_results(&buffer_init, &buffer_result, 
        KERNEL_AND_BUFFER_ITERS + KERNEL_AND_BUFFER_ITERS + KERNEL_RUN_ITERS);

    // ##################################################
    // ############# CAUTION IS OVERRATED ###############
    // ##################################################

    print!("\n");
    println!("Enqueuing {} oh-fuck-it kernel buffer sequences... ", KERNEL_AND_BUFFER_ITERS);

    let kern_buf_start = Instant::now();

    // let mut kern_events = EventList::new();
    // let mut buf_events = EventList::new();

    for _ in 0..KERNEL_AND_BUFFER_ITERS {
        kern.enqueue_with(None, None, Some(&mut kern_events)).unwrap();
        unsafe { buffer_result.enqueue_fill_vec(false, None, Some(&mut buf_events)).unwrap(); }
    }

    print_elapsed("queue unfinished", kern_buf_start);
    ocl_pq.queue().finish();    
    print_elapsed("queue finished", kern_buf_start);

    verify_results(&buffer_init, &buffer_result, 
        KERNEL_AND_BUFFER_ITERS + KERNEL_AND_BUFFER_ITERS + KERNEL_AND_BUFFER_ITERS + KERNEL_RUN_ITERS);
}


fn print_elapsed(title: &str, start: Instant) {
    let time_elapsed = Instant::now().duration_from_earlier(start);
    let elapsed_ms = time_elapsed.subsec_nanos() / 1000000;
    let separator = if title.len() > 0 { ": " } else { "" };
    println!("    {}{}: {}.{:03}", title, separator, time_elapsed.as_secs(), elapsed_ms);
}


fn verify_results(buffer_init: &Buffer<f32>, buffer_result: &Buffer<f32>, iters: i32) {
    print!("\nVerifying result values... ");
    if PRINT_SOME_RESULTS { print!("(printing {})\n", RESULTS_TO_PRINT); }

    // let margin_of_error = iters as f32 / 100000.0;
    let margin_of_error = 0.1 as f32;

    for idx in 0..DATASET_SIZE {
        let correct = buffer_init[idx] + (iters as f32 * SCALAR);
        // let correct = buffer_init[i] + SCALAR;
        assert!((correct - buffer_result[idx]).abs() < margin_of_error, 
            "    INVALID RESULT[{}]: init: {}, correct: {}, margin: {}, result: {}", 
            idx, buffer_init[idx], correct, margin_of_error, buffer_result[idx]);

        if PRINT_SOME_RESULTS && (idx % (DATASET_SIZE / RESULTS_TO_PRINT)) == 0  {
            println!("    [{}]: init: {}, correct: {}, result: {}", idx, buffer_init[idx],
                correct, buffer_result[idx]);
        }
    }

    if PRINT_SOME_RESULTS { print!("\n"); }
    println!("All result values are correct.");
}
