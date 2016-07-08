local etcd_url = 'http://' .. os.getenv('HOST_IP') .. ':2379'
local etcd = require('etcd').new(etcd_url)

lustache = require "lustache"


