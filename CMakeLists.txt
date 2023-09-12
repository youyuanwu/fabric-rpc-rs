cmake_minimum_required(VERSION 3.20)

project(service-fabric LANGUAGES)

# Get-FileHash .\build\_deps\protoc-src\bin\protoc.exe -Algorithm SHA256

# for ide to work add:
# "rust-analyzer.cargo.buildScripts.enable": true

message(STATUS "fetching protoc")
include(FetchContent)
FetchContent_Declare(
  protoc
  DOWNLOAD_NO_EXTRACT FALSE
  DOWNLOAD_EXTRACT_TIMESTAMP TRUE
  URL https://github.com/protocolbuffers/protobuf/releases/download/v24.1/protoc-24.1-win64.zip
  URL_HASH SHA256=DAD1EFED75CB772A0C9E5D4646006BF87E1E051191473A2A30C9DB66EE223CFD
)
FetchContent_GetProperties(protoc)
if(NOT protoc_POPULATED)
  FetchContent_Populate(protoc)
endif()