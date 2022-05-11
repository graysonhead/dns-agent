# dns-agent

[![Rust](https://github.com/graysonhead/dns-agent/actions/workflows/rust.yml/badge.svg)](https://github.com/graysonhead/dns-agent/actions/workflows/rust.yml)

dns-agent is an agent that can be run by a systemd timer, cron, or another scheduling program at regular intervals to allow a system to update it's own IP address.

In addition to allowing for Dynamic DNS for DHCP issued external IP addresses, it can also be used to make use of DNS in IPv6 environments that make use of SLAAC.

## Backends

Right now, only Digital Ocean is supported, but I'm planning on adding PowerDNS. Right now the DNS Provider Api is a bit of a mess but I'm planning on cleaning it up once I add a second provider. In the meantime, I'll happliy take a look at any pull requests if there is enough interest.

## Install



