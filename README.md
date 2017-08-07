# rs-collector

![Release Status](https://img.shields.io/badge/status-alpha-red.svg) [![Linux & OS X Build Status](https://img.shields.io/travis/lukaspustina/rs-collector/master.svg)](https://travis-ci.org/lukaspustina/rs-collector) [![GitHub release](https://img.shields.io/github/release/lukaspustina/rs-collector.svg)](https://github.com/lukaspustina/rs-collector/releases) [![](https://img.shields.io/crates/v/rs-collector.svg)](https://crates.io/crates/rs-collector) [![license](https://img.shields.io/github/license/lukaspustina/rs-collector.svg)](https://github.com/lukaspustina/rs-collector/blob/master/LICENSE) [![Ansible Role](https://img.shields.io/badge/ansible--galaxy-rs__collector-blue.svg)](https://galaxy.ansible.com/Rheinwerk/rs_collector/)


_rs-collector_ is a [Bosun](https://bosun.org) compatible collector for various services that are not covered by [scollector](https://bosun.org/scollector/) and that we use at [CenterDevice](https://www.centerdevice.de/en/).

**Attention**:  Please be advised, even though we are running _rs-collector_ on our production systems successfully for months, this is no stable software.

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
**Table of Contents**  *generated with [DocToc](https://github.com/thlorenz/doctoc)*

- [Collectors](#collectors)
  - [Galera](#galera)
    - [Example Alarms](#example-alarms)
  - [HasIpAddr](#hasipaddr)
    - [Example Alarm](#example-alarm)
  - [JVM](#jvm)
  - [Mongo](#mongo)
    - [Example Alarms](#example-alarms-1)
  - [Postfix](#postfix)
    - [Example Alarms](#example-alarms-2)
  - [rs-collector Internal Metrics](#rs-collector-internal-metrics)
- [Configuration](#configuration)
- [Installation](#installation)
  - [Ubuntu [x86_64 and Raspberry Pi]](#ubuntu-x86_64-and-raspberry-pi)
  - [Binary](#binary)
  - [Sources](#sources)
  - [Ansible](#ansible)
- [Know Issues](#know-issues)
- [Roadmap](#roadmap)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->


## Collectors

1. [Galera](#galera) - Collects metrics about the cluster status and cluster sync performance of a Percona Galera MySQL cluster.
1. [HasIpAddr](#hasipaddr) - Checks if a host has bound specific IPv4 addresses.
1. [JVM](#JVM) - Collects garbage collection statistics.
1. [MongoDB](#mongo) - Collects replicaset metrics.
1. [Postfix](#postfix) - Collects queue lengths for all postfix queues.
1. [rs-collector](#rs-collector) - Collects internal metrics for rs-collector.

See below for details about the collectors.

### Galera

The _Galera_ collector collects metrics about the cluster status and cluster sync performance of a Percona Galera MySQL cluster. We use it to monitor cluster split brain and general degradation situation. There is a full list of all available metrics in [galera.rs](src/collectors/galera.rs), function `metadata`.

#### Example Alarms

```
alert galera.cluster.state.uuid.no.consensus {
  template = ...
  critNotification = default

  $metric = avg:galera.wsrep.cluster.state.uuid{domain=wildcard(*)}
  $q=q("$metric", "5m", "")
  $a = avg($q)
  $f = first($q)
  $q_alert = ($a - $f) != 0
  crit = $q_alert
}

alert galera.cluster.state.not.primary {
  template = ...
  critNotification = default

  $metric = sum:galera.wsrep.cluster.status{host=wildcard(*),domain=wildcard(*)}
  $q = q("$metric", "5m", "")
  $t = t(last($q), "domain")
  $q_alert = sum($t)
  $primaryValue = 0
  crit = $q_alert != $primaryValue
}

alert galera.local.state.not.synced {
  template = ...
  critNotification = default

  $metric = zimsum:5m-avg:galera.wsrep.local.state{domain=wildcard(*)}
  $q = q("$metric", "5m", "")
  $q_alert = last($q)
  $syncedValue = 12
  crit = $q_alert != $syncedValue
}

alert galera.cluster.size.degraded {
  template = ...
  critNotification = default

  $metric = avg:galera.wsrep.cluster.size{domain=wildcard(*)}
  $q = q("$metric", "5m", "")
  $q_alert = last($q)
  $critValue = 3
  crit = $q_alert != $critValue
}
```


### HasIpAddr

The _HasIpAddr_ collector sends either 1 or 0 if a host has bound a specific IPv4 address or not, respectively. This is helpful in cases where hosts bind or release IPv4 addresses dynamically. For example, in a `keepalived` VRRP cluster it allows Bosun to check if and on how many hosts a virtual, high available IP address is bound.

In our production clusters we have observed situations when none of the cluster members has bound the virtual IP address. This collector allows us to define an alarm for such cases.

#### Example Alarm

```
alert os.net.vrrp-vip-failed {
  template = ...
  critNotification = default

  $metric = sum:os.net.has_ipv4s{ipv4=wildcard(*)}

  $q_alert = sum(t(last(q("$metric", "5m", "")), "ipv4"))

  $expected = 1
  $critValue = $expected
  crit = $q_alert != $critValue
}
```

### JVM

The _JVM_ collector collects garbage collection statistics, i.e., those that `jstat -gc` reveals for each specified, running JVM. This collector has been tested with OpenJDK "7u51-2.4.6-1ubuntu4" and Oravle JDK "1.8.0_121". JVMs are identified by a regular expression that matches the class name or the command line arguments and ass

This collector only collects statistics for specified JVM; cf. example configuration. It currently does not distinguish between multiple instances of the same identified JVM. 

### Mongo

The _Mongo_ collector collects MongoDB replicaset and cluster metrics. We use it to monitor cluster split brain and general degradation situation. There is a full list of all available metrics in [mongo.rs](src/collectors/mongo.rs), function `metadata`.

Especially the following two metrics are helpful:

* `mongo.replicasets.members.mystate` collects the "myState" variable from each replica set member. This allows to compute if the particular replica set is in a sane state.
* `mongo.replicasets.oplog_lag.[min,avg,max]` collects the min, avg, and max oplog replication lag between a replica set's primary and the corresponding secondaries. These values are measured only on the currently active primary.

#### Example Alarms

```
alert mongo.replicaset.state.unexpected {
  template = ...
  critNotification = default

  $metric = sum:mongo.replicasets.members.mystate{host=wildcard(*),replicaset=wildcard(*)}
  $q = q("$metric", "5m", "")
  $t = t(last($q), "replicaset")
  $q_alert = sum($t)
  $critValue = 5
  crit = $q_alert != $critValue
}
```

### Postfix

The _Postfix_ collector collects metrics about Postfix' queues. This is helpful to monitor how the queues fill and empty over time as well as if the queues are emptied at all in order to alarm when mail delivery stalls. There is a full list of all available metrics in [postfix.rs](src/collectors/postfix.rs), function `metadata`.

#### Example Alarms

```
alert postfix.mailqueue.deferred.too.long {
  template = ...
  critNotification = default
  warnNotification = default

  $metric = sum:5m-min:postfix.queues.deferred{domain=wildcard(*)}
  $q = q("$metric", "5m", "")
  $t = t(last($q), "domain")
  $q_alert = sum($t)
}

alert postfix.mailqueue.deferred.unchanged {
  template = ...
  warnNotification = default

  $period = 4h
  $metric = postfix.queues.deferred{domain=wildcard(*)}
  $q_min = q("min:$metric", "$period", "")
  $q_max = q("max:$metric", "$period", "")

  $min_queue_len = min($q_min)
  $max_queue_len = max($q_max)

  $q_alert = $min_queue_len > 0 && $max_queue_len == $min_queue_len
  warn = $q_alert
}
```

### rs-collector Internal Metrics
* `rs-collector.stats.rss` collects the resident set size (physical memory) in KB consumed by rs-collector; only supported on Linux.
* `rs-collector.stats.samples` collects the number of transmitted samples.
* `rs-collector.versio` collects the version 'x.y.z' of rs-collector as x * 1.000.0000 + y * 1000 + z.

These metrics can also be used to check the liveliness of rs-collector and as a heartbeat.


## Configuration

Please see this [example](examples/rs-collector.conf).


## Installation

### Ubuntu [x86_64 and Raspberry Pi]

Pleae add my [PackageCloud](https://packagecloud.io/lukaspustina/opensource) open source repository and install _rs-collector_ via apt.

```bash
curl -s https://packagecloud.io/install/repositories/lukaspustina/opensource/script.deb.sh | sudo bash
sudo apt-get install rs-collector
```

### Binaries [x86_64 and Raspberry Pi]

There are binaries available at the GitHub [release page](https://github.com/lukaspustina/rs-collector/releases) compiled on Ubuntu.

### Sources

Please install Rust via [rustup](https://www.rustup.rs) and then run

```bash
cargo install rs-collector
```

### Ansible

There is also an Ansible role available at [Ansible Galaxy](https://galaxy.ansible.com/Rheinwerk/rs_collector/) that automates the installation of _rs-collector_.


## Know Issues

* General: Minor memory leak in chan::tick -- cf. [Roadmap](TODO.md).

* JVM: Does not distinguish between JVMs with the same name assigned via configuration, i.e., multiples instances of the same Java application.


## Roadmap

Please see [Todos](TODO.md).

