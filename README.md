# rs-collector

![Release Status](https://img.shields.io/badge/status-alpha-red.svg) [![Build Status](https://img.shields.io/travis/lukaspustina/rs-collector/master.svg)](https://travis-ci.org/lukaspustina/rs-collector) [![GitHub release](https://img.shields.io/github/release/lukaspustina/rs-collector.svg)](https://github.com/lukaspustina/rs-collector/releases) [![license](https://img.shields.io/github/license/lukaspustina/rs-collector.svg)](https://github.com/lukaspustina/rs-collector/blob/master/LICENSE) [![Ansible Role](https://img.shields.io/badge/ansible--galaxy-rs__collector-blue.svg)](https://galaxy.ansible.com/Rheinwerk/rs_collector/)


## Roadmap

Please see [Todos](TODO.md).

## Collectors

1. [Galera](#galera) -- Collects metrics about the cluster status and cluster sync performance of a Percona Galera MySQL cluster.
1. [HasIpAddr](#hasipaddr) -- Checks if a host has bound specified IPv4 address.

See below for details about the collectors.

### Galera
### HasIpAddr

_HasIpAddr_ sends either 1 or 0 if a host has bound a specific IPv4 address or not, respectively. This is helpful in cases where hosts bind or release IPv4 addresses dynamically. For example, in a `keepalived` VRRP cluster it allows Bosun to check if and on how many hosts a virtual, high available IP address is bound.

In our production cluster we have observed situations when none of the cluster members has bound the virtual IP address. This collector allows us to define an alarm for such cases.


## Configuration

Please see this [example](examples/rs-collector.conf).


## Releases

Travis CI creates Ubuntu Trusty packages for each release. Please see the [Repository](https://packagecloud.io/lukaspustina/opensource) for details.

### Workflow

1. Push and wait for Travis CI to finish master build.
1. Increase Cargo version.
1. Tag master with corresponding version tag.
1. Push tags.


## Deploy to Production

`ansible-playbook mysql-servers.yml --tags rs_collector --diff --extra-vars "RW_APT_CACHE_UPDATE=true RW_ENABLE_DOWNLOADS=true"`

## Installation

There is also an Ansile role available at [Ansible Galaxy](https://galaxy.ansible.com/Rheinwerk/rs_collector/) that automates the installation of rs-collector.
