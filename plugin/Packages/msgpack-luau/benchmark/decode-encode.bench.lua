local msgpack = require(game.ReplicatedStorage.msgpack)

local httpService = game:GetService("HttpService")
local msgpackDecode = msgpack.decode
local msgpackEncode = msgpack.encode

local jsonMessage = require(game.ServerStorage.JsonMessage)
local msgpackMessage = require(game.ServerStorage.MsgpackMessage)

return {

  ParameterGenerator = function() end,

  Functions = {
    ["JSONDecode & JSONEncode"] = function(Profiler)
      Profiler.Begin("JSONDecode")
      local x = httpService:JSONDecode(jsonMessage)
      Profiler.End()
      Profiler.Begin("JSONEncode")
      httpService:JSONEncode(x)
      Profiler.End()
    end,

    ["msgpack.decode & msgpack.encode"] = function(Profiler)
      Profiler.Begin("msgpack.decode")
      local x = msgpackDecode(msgpackMessage)
      Profiler.End()
      Profiler.Begin("msgpack.encode")
      msgpackEncode(x)
      Profiler.End()
    end
  }

}
