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
    python3 python3-pip

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
