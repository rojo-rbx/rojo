local msgpack = require(game.ReplicatedStorage.msgpack)

local httpService = game:GetService("HttpService")
local msgpackEncode = msgpack.encode

local jsonMessage = require(game.ServerStorage.JsonMessage)
local data = httpService:JSONDecode(jsonMessage)

return {

  ParameterGenerator = function() end,

  Functions = {
    ["JSONEncode"] = function(Profiler)
      httpService:JSONEncode(data)
    end,

    ["msgpack.encode"] = function(Profiler)
      msgpackEncode(data)
    end
  }

}
