# dns-agent

[![Rust](https://github.com/graysonhead/dns-agent/actions/workflows/rust.yml/badge.svg)](https://github.com/graysonhead/dns-agent/actions/workflows/rust.yml)

dns-agent is an agent that can be run by a systemd timer, cron, or another scheduling program at regular intervals to allow a system to update it's own IP address.

In addition to allowing for Dynamic DNS for DHCP issued external IP addresses, it can also be used to more easily utilize DNS in IPv6 SLAAC networks (or in dynamic networks where you don't control the DHCP server).

## Backends

Currently, Digital Ocean and Cloudflare are the only supported backends. New backends should be relatively easy to add by implementing the DnsBackend trait. Pull requests are welcomed.

## Examples

See the `examples` directory for configuration examples.
