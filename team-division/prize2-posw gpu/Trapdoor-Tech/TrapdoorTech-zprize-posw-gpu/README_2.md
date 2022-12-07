# Accelerating PoSW on Quadro RTX 4000

Prize Sponsor: Aleo

Prize Architect: Aleo  
Prize Reward: 3M Aleo Credits

## Run the benchmark

To maximize the adjusted score, we choose another setup of following hardware using **NVIDIA Quadro RTX 4000** as GPU, the total cost would be **$0.30/hour**. We achieved **524** proofs generation in 20 seconds.

* 4 vCPUs on AMD EPYC 7282 16 core Processor
* 4G memory
* 40G storage space
* a single **Nvidia Quadro RTX 4000** GPU card with 8G memory
* Ubuntu 22.04 LTS

A comparison is made for NVIDIA RTX A5000 GPU and NVIDIA Quadro RTX 4000 GPU:

| CPU                      |                    GPU                    | Mmeory | Storage | System Version | Proofs (in 20 s) | Price | Adjusted Score |
| :----------------------- | :---------------------------------------: | :----: | :-----: | :------------: | :--------------: | :---: | :------------: |
| 1 NVIDIA RTX A5000       |     6 cores (AMD EPYC 7413 Processor)     |   6G   |   40G   |  Ubuntu 20.04  |       1130       | 0.86  |   **13.14**    |
| 1 NVIDIA Quadro RTX 4000 | 4 cores (AMD EPYC 7282 16 core Processor) |   4G   |   40G   |  Ubuntu 22.04  |       524        | 0.30  |   **17.46**    |


***For the details of our optimizations, please check our
[ZPrize - POSW GPU optimization.pdf](./ZPrize%20-%20POSW%20GPU%20optimization.pdf)***

To run the benchmark, please follow these steps:

1. Install Nvidia Cuda 11-7:	`sudo apt install cuda-11-7`
2. Add nvcc into $PATH:			`export PATH=$PATH:/usr/local/cuda/bin`
3. Regenerate GPU kernel file:	`cd ../TrapdoorTech-zprize-ec-gpu-kernels; cargo run -- --cuda`
4. Copy the newly generated GPU kernel file back: `cd ../TrapdoorTech-zprize-posw-gpu; cp ../TrapdoorTech-zprize-ec-gpu-kernels/gpu-kernel.bin .`
5. Generate all GPU lookup tables by running: `cargo run --bin generator --release -- --window-size 7`, for Nvidia Quadro RTX 4000 GPU, we recommend setting `window size = 7`.
6. Since the SRS file is no longer available on the Internet, we uploaded it for convenience, please move it to dedicated repository: `mkdir -p ~/.aleo/resources; mv posw.proving.7b5709f ~/.aleo/resources`
7. Run to test TPS: `THREAD_COUNT=9 cargo run --release`, `THREAD_COUNT` specifies the number of prover threads and `9` is an empirical value. 

Note:

1. if `lagrange-g-calced.data` file is corrupted, you can regenerate it along with lookup tables, by running `cargo run --bin generator --release -- --window-size 7 --gen-shifted-lagrange-basis`
