# rs-collector

[![Build Status](https://travis-ci.org/lukaspustina/rs-collector.svg?branch=master)](https://travis-ci.org/lukaspustina/rs-collector)

## Todo

1. [x] Fix memory leak in chan::tick
    cf. [https://github.com/BurntSushi/chan/issues/11](https://github.com/BurntSushi/chan/issues/11) and branch mem_leak_chan_tick.

    This is known behaviour of chan::tick. The memory leak led to 236 KB
    allocated memory during 64 hours. That doesn't hurt too much
    especially because we plan to restart the process every 24 because
    of log rotation. Skipping this for now.

1. [x] Transform all wsrep values to metrics
1. [x] Add metadata for all wsrep values
1. [x] Check additional state for metrics
1. [x] Handle tags
1. [x] Automate packaging for Ubuntu
  1. [x] Ansible Role
  1. [x] Update Readme: Link to package and Ansible role
1. [ ] Reconnect Logic for Galera Collector
1. [ ] Tests
1. [ ] Extend bosun_emitter to send multiple data points
1. [ ] Failure Modes
  1. Check if Collector is alive
  1. Remove collector if dead
  1. Remove collector if too many collection failures
1. [ ] Support multiple Galera Collectors -- also change in Ansible role
1. [ ] Add internal metrics `rs-collector.*`


## Installation

There is an Ansile role available at [Ansible Galaxy](https://galaxy.ansible.com/Rheinwerk/rs_collector/) that automates the installation of rs-collector.

