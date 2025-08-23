FROM mcr.microsoft.com/devcontainers/rust:0-1-bullseye
#FROM ubuntu:latest
LABEL authors="y_mc"

#ENTRYPOINT ["top", "-b"]
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
       && apt-get -y install clang lld \
       && apt-get autoremove -y && apt-get clean -y