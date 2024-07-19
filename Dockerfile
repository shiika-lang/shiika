# Use an official Ubuntu as a parent image
FROM ubuntu:22.04

# Configure the environment
ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/root/.cargo/bin:${PATH}"
ENV LLC=llc-16
ENV CLANG=clang-16

# Install necessary packages
RUN apt-get update && apt-get install -y \
    git \
    curl \
    build-essential \
    cmake \
    libgc-dev \
    wget \
    gnupg \
    software-properties-common \
    zlib1g-dev \
    libzstd-dev \
    && rm -rf /var/lib/apt/lists/*

# Add the LLVM 16 repository and install LLVM 16
RUN wget https://apt.llvm.org/llvm.sh && chmod +x llvm.sh && ./llvm.sh 16

# Install Polly and other necessary LLVM packages
RUN apt-get update && apt-get install -y \
    libpolly-16-dev \
    llvm-16-dev \
    libllvm16 \
    liblld-16-dev \
    libclang-16-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Set the working directory
WORKDIR /app

# Set the default command
CMD ["/bin/bash"]
