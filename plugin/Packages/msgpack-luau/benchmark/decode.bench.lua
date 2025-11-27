local msgpack = require(game.ReplicatedStorage.msgpack)

local httpService = game:GetService("HttpService")
local msgpackDecode = msgpack.decode

local jsonMessage = require(game.ServerStorage.JsonMessage)
local msgpackMessage = require(game.ServerStorage.MsgpackMessage)

return {

  ParameterGenerator = function() end,

  Functions = {
    ["JSONDecode"] = function(Profiler)
      httpService:JSONDecode(jsonMessage)
    end,

    ["msgpack.decode"] = function(Profiler)
      msgpackDecode(msgpackMessage)
    end
  }

}
