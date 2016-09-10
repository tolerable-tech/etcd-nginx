CHANGELOG
=========

v0.1.1
======

use v0.1.2 of `conf_manager` which sorts `upstreams` and then sorts both
`ComponentSpec`'s and `StaticSpec`'s by name before generating confs to stop
the specs from constantly changing since Etcd provides no guarantees on key
ordering when listing directories.

