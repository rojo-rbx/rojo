local Rojo = script:FindFirstAncestor("Rojo")
local invariant = require(script.Parent.Parent.invariant)

local Log = require(Rojo.Packages.Log)

local function fetchInstances(idList, instanceMap, apiContext)
	return apiContext:fetch(idList)
		:andThen(function(body: {sessionId: string, path: string})
			-- The endpoint `api/fetech/idlist` returns a table that contains
			-- `sessionId` and `path`. The value of `path` is the name of a
			-- file in the Content folder that may be loaded via `GetObjects`.
			local objects = game:GetObjects("rbxasset://" .. body.path)
			-- `ReferentMap` is a folder that contains nothing but
			-- ObjectValues which will be named after entries in `instanceMap`
			-- and have their `Value` property point towards a new Instance
			-- that it can be swapped out with. In turn, `reified` is a
			-- container for all of the objects pointed to by those instances.
			local map = objects[1]:FindFirstChild("ReferentMap")
			local reified = objects[1]:FindFirstChild("Reified")
			if map == nil then
				invariant("The fetch endpoint returned a malformed folder: missing ReferentMap")
			end
			if reified == nil then
				invariant("The fetch endpoint returned a malformed folder: missing Reified")
			end
			for _, entry in map:GetChildren() do
				if entry:IsA("ObjectValue") then
					local key, value = entry.Name, entry.Value
					if value == nil or not value:IsDescendantOf(reified) then
						invariant("ReferentMap contained entry {} that was parented to an outside source", key)
					else
						-- This could be a problem if Roblox ever supports
						-- parallel access to the DataModel but right now,
						-- there's no way this results in a data race.
						local oldInstance: Instance = instanceMap.fromIds[key]
						instanceMap:insert(key, value)
						Log.trace("Swapping Instance {} out", key)

						local oldParent = oldInstance.Parent
						local children = oldInstance:GetChildren()
						for _, child in children do
							child.Parent = value
						end
						value.Parent = oldParent

						-- So long and thanks for all the fish :-)
						oldInstance:Destroy()
					end
				else
					invariant("ReferentMap entry `{}` was a `{}` and not an ObjectValue", entry.Name, entry.ClassName)
				end
			end
		end)
end

return fetchInstances