# TODOs

## rs-collector

- [x] Fix memory leak in chan::tick
  cf. [https://github.com/BurntSushi/chan/issues/11](https://github.com/BurntSushi/chan/issues/11) and branch mem_leak_chan_tick.

  This is known behaviour of chan::tick. The memory leak led to 236 KB allocated memory during 64 hours. That doesn't hurt too much especially because we plan to restart the process every 24 because of log rotation. Skipping this for now.

- [x] Transform all wsrep values to metrics
- [x] Add metadata for all wsrep values
- [x] Check additional state for metrics
- [x] Handle tags
- [x] Automate packaging for Ubuntu
  - [x] Ansible Role
  - [x] Update Readme: Link to package and Ansible role
- [ ] Failure Modes
  - [x] Reinitialize collector if collection fails.
    - [x] Reconnect Logic for Galera Collector
  - [ ] Remove collector if too many collection failures.
  - [ ] Remove collector if collection thread does not respond anymore.
- [x] Add timestamps to log messages
- [ ] Tests
- [ ] Fix Todos
- [ ] Rust documentation
- [ ] Enhance deb package
  - [ ] Don't overwrite changed config files
- [ ] Move project to Rheinwerk
- [ ] Extend bosun_emitter to send multiple data points
- [ ] Support multiple Galera Collectors -- also change in Ansible role
- [ ] Add internal metrics `rs-collector.*`

## Collectors

- [x] Check for IP bound to interface -- keepalived VIP side effect
- [ ] Postfix metrics
- [ ] MongoDB replication metrics
- [ ] Docker
- [ ] ifconfig / network inferface frame metrics

- [ ] Ceph metrics
- [ ] MySQL performance metrics
- [ ] MongoDB performance metrics
- [ ] Tomcat management servlet metrics
- [ ] LACP / interface bond metrics

