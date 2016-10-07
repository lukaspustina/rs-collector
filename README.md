# rs-collector

## Todo

1. Fix memory leak in chan::tick
    cf. [https://github.com/BurntSushi/chan/issues/11](https://github.com/BurntSushi/chan/issues/11) and branch mem_leak_chan_tick.

    This is known behaviour of chan::tick and we need to find a work around.

1. Transform all wsrep values to metrics
1. Handle tags
1. Automate packaging for Ubuntu
1. Tests
1. Extend bosun_emitter to send multiple data points

