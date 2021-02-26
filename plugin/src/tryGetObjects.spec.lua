-- TODO: Have some documentation/script in order to force the rbxassets to be there
return function()
	local InstanceMap = require(script.Parent.InstanceMap)
	local PatchSet = require(script.Parent.PatchSet)
	local Promise = require(script.Parent.Parent.Promise)
	local tryGetObjects = require(script.Parent.tryGetObjects)

	local MESH_DOGE = "rbxassetid://4574885352"

	it("should apply updates to existing instances", function()
		local mockApiContext = {}
		function mockApiContext:createAssets()
			return Promise.resolve("rbxasset://rojo-tests/DogeUpdate.rbxm")
		end

		local instanceMap = InstanceMap.new()

		local oldDoge = Instance.new("MeshPart")
		instanceMap:insert("DOGE", oldDoge)

		local patch = PatchSet.newEmpty()
		table.insert(patch.updated, {
			id = "DOGE",
			changedProperties = {
				MeshId = {
					Type = "Content",
					Value = MESH_DOGE,
				},
			},
		})

		local _, unappliedPatch = assert(tryGetObjects(instanceMap, mockApiContext, patch):await())
		assert(PatchSet.isEmpty(unappliedPatch), "expected remaining patch to be empty")

		local newDoge = instanceMap.fromIds["DOGE"]
		assert(newDoge ~= nil, "no instance with id DOGE")
		expect(newDoge.MeshId).to.equal(MESH_DOGE)
	end)
end
