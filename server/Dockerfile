# Stage 1: Build the Rust application
FROM rust:1.75.0-slim-bullseye AS build

WORKDIR /app

# Copy your project files (assuming your project root is the context for the build)
COPY . . 

# Build the release version of your application
RUN cargo build --release

# Stage 2: Create the final runtime image
FROM debian:bullseye-slim

# Install necessary dependencies (you might need more depending on your app)
RUN apt-get update && apt-get install -y openssl ca-certificates

# Create a non-root user for running the application
ARG USER_ID=1001
RUN useradd -u $USER_ID -ms /bin/bash appuser 
USER appuser

# Create a directory for your application 
WORKDIR /app 

# Copy the compiled binary from the build stage
COPY --from=build /app/target/release/service-http /app/service-http 

# Expose the port your server listens on 
EXPOSE 3000

# Environment variable for MongoDB connection (replace with your actual URI)
ENV MONGODB_URI=mongodb://your_mongodb_host:27017/your_database_name

# Command to start the server
CMD ["/app/service-http"]