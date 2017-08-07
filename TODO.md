# TODOs

## rs-collector

- [x] Fix memory leak in chan::tick
  cf. [https://github.com/BurntSushi/chan/issues/11](https://github.com/BurntSushi/chan/issues/11) and branch mem_leak_chan_tick.

  This is known behavior of chan::tick. The memory leak led to 236 KB allocated memory during 64 hours. That doesn't hurt too much especially because we restart the process every 24 for log rotation. Skipping this for now.

- [x] Transform all wsrep values to metrics
- [x] Add metadata for all wsrep values
- [x] Check additional state for metrics
- [x] Handle tags
- [x] Automate packaging for Ubuntu
  - [x] Ansible Role
  - [x] Update Readme: Link to package and Ansible role
- [ ] Redo collectors as real state machine
- [+] Failure Modes
  - [x] Reinitialize collector if collection fails.
    - [x] Reconnect Logic for Galera Collector
  - [ ] Remove collector if too many collection failures.
  - [ ] Remove collector if collection thread does not respond anymore.
- [x] Add timestamps to log messages
- [ ] Tests
- [ ] Clean up
  - https://llogiq.github.io/2016/02/11/rustic.html
- [ ] Make it safe
  - [ ] Clippy-fy
  - [ ] Fix Todos
  - [ ] Eliminate unwraps
- [ ] Rust documentation
- [x] Enhance deb package
  - [x] Don't overwrite changed config files
- [ ] Move project to Rheinwerk
- [ ] Extend bosun_emitter to send multiple data points
- [ ] Support multiple Galera Collectors -- also change in Ansible role

## Collectors

- [x] Check for IP bound to interface -- keepalived VIP side effect
- [+] Postfix metrics
  - [x] Queue len
  - [ ] Send / Recv stats
- [+] MongoDB
  - [+] replication metrics -- cf. [replSetGetStatus](https://docs.mongodb.com/manual/reference/command/replSetGetStatus/)
    - [x] myState (A)
    - [+] Oplog replication lag (A)
      - [ ] Explain lag spikes due to idle times -- cf. [Mongo documentation](https://docs.mongodb.com/manual/tutorial/troubleshoot-replica-sets/#check-the-replication-lag)
      - [ ] Show alert example
    - [ ] Heartbeat latency = lastHeartbeatRecv - lastHeartbeat (A)
    - [ ] roundtrip time = pingMs
    - [ ] uptime = uptime -> Rate
    - [ ] health = health only from point of view of primary (A)
  - [ ] Balancer Status
  - [ ] other metrics?
- [x] Internal metrics `rs-collector.*`
  - [x] Version --  can also be used to check liveliness and as heartbeat
  - [x] Number of transmitted samples -- can also be used to check liveliness and as heartbeat
  - [x] RSS cf. [procinfo](https://danburkert.github.io/procinfo-rs/procinfo/pid/struct.Status.html) -- can also be used to check liveliness and as heartbeat
- [ ] Docker
  - [ ] Use [rust-docker](https://github.com/ghmlee/rust-docker)
- [ ] ifconfig / network inferface frame metrics
- [ ] DNS
  - [ ] Serial numbers of all authoritive servers

## One day, maybe

- [ ] Ceph metrics
- [ ] MySQL performance metrics
- [ ] MongoDB performance metrics
- [ ] Tomcat management servlet metrics
- [ ] LACP / interface bond metrics

