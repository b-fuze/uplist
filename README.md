# uplist - basic static file server with upload

<p align="center">
  <img src="https://raw.githubusercontent.com/b-fuze/uplist/master/assets/upload-server.gif" />
</p>

A single-binary HTTP static-file server with a basic upload feature built with
[Warp](https://github.com/seanmonstar/warp).

## Disclaimer
This is not a secure and hardened server, it's only meant to be a quick and
dirty static file server. If you need something robust I suggest [Nginx](https://nginx.org/).

## Features

 - Single binary
 - Small
 - Efficient
 - Async
 - Can upload

## Usage
```
USAGE
  uplist [PORT [MAX_UPLOAD]]

DESCRIPTION
  Simple HTTP-based file upload service.

OPTIONS
  PORT
    Port to listen on. Defaults to 8000

  MAX_UPLOAD
    Maximum file upload size in bytes. Defaults to 500MB
```

## Building
Get [Rust](https://www.rust-lang.org/tools/install) if you haven't already, then run
```
cargo build --release
```

