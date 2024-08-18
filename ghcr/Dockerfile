# Use a base image with Ubuntu
FROM ubuntu:latest

# Install TeX Live and any additional packages
RUN apt-get update && \
    apt-get install -y \
    graphviz \
    git \
    curl \
    make \
    build-essential \
    python3 python3-pip \
    libnss3 \
    libnspr4 \
    libdbus-1-3 \
    libatk1.0-0t64 \
    libatk-bridge2.0-0t64 \
    libcups2t64 \
    libdrm2 \
    libxkbcommon0 \
    libatspi2.0-0t64 \
    libxcomposite1 \
    libxdamage1 \
    libxfixes3 \
    libxrandr2 \
    libgbm1 \
    libasound2t64

RUN pip install --break-system-packages ansitoimg

# Set up the working directory
WORKDIR /workspace

# Ensure that the working directory has appropriate permissions
RUN chown -R $(id -u):$(id -g) /workspace

# Create a non-root user and set permissions
RUN useradd -m -s /bin/bash myuser && \
    chown -R myuser:myuser /workspace

# Switch to the non-root user
USER myuser

# Default command (you can override this in the GitHub Actions workflow)
CMD ["bash"]
