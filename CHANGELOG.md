# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.8] - 2020-03-16

### Fixed
- jvm collector now properly collects metrics for processes with a PID greater than 65535
- panics will now abort the application to avoid dead collectors or a dead bosun emitter going unnoticed


## [0.1.7] - 2018-06-18

### Added
- Megaraid collector


## [0.1.6] - 2018-05-29

### Added
- MongoDB connections stats and op counters

### Changed
- MongoDB collector automatically recognizes if replSetGetStatus is suitable. In case of mongos or non replicated mongod, this metric is omitted.


[Unreleased]: https://github.com/lukaspustina/ceres/compare/v0.1.8...HEAD
[v0.1.8]: https://github.com/lukaspustina/ceres/compare/v0.1.7...v0.1.8
[v0.1.7]: https://github.com/lukaspustina/ceres/compare/v0.1.6...v0.1.7
[v0.1.6]: https://github.com/lukaspustina/ceres/compare/v0.1.5...v0.1.6
