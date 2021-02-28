return function()
	local InstanceMap = require(script.Parent.InstanceMap)
	local PatchSet = require(script.Parent.PatchSet)
	local Promise = require(script.Parent.Parent.Promise)
	local tryGetObjects = require(script.Parent.tryGetObjects)

	-- game:GetObjects(ASSET_DOGE) == { Doge mesh }
	local ASSET_DOGE = "rbxassetid://4574885387"

	-- This is the actual Mesh ID for Doge
	local MESH_DOGE = "rbxassetid://4574885352"

	local function createMockApiContext()
		local mockApiContext = {}
		function mockApiContext:createAssets()
			return Promise.resolve(ASSET_DOGE)
		end

		return mockApiContext
	end

	it("should apply updates to existing instances", function()
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

		local _, unappliedPatch = assert(
			tryGetObjects(instanceMap, createMockApiContext(), patch)
				:await()
		)

		assert(PatchSet.isEmpty(unappliedPatch), "expected remaining patch to be empty")

		local newDoge = instanceMap.fromIds["DOGE"]
		assert(newDoge ~= nil, "no instance with id DOGE")
		expect(newDoge.MeshId).to.equal(MESH_DOGE)
	end)

	it("should be able to create new instances", function()
		local instanceMap = InstanceMap.new()

		local folder = Instance.new("Folder")
		instanceMap:insert("PARENT", folder)

		local child = Instance.new("Folder")
		child.Name = "Child"
		instanceMap:insert("CHILD", child)

		local patch = PatchSet.newEmpty()

		patch.added["DOGE"] = {
			Id = "DOGE",
			Name = "Doge",
			ClassName = "MeshPart",
			Parent = "PARENT",
			Properties = {
				MeshId = {
					Type = "Content",
					Value = MESH_DOGE,
				},
			},
			Children = { "CHILD" },
		}

		local _, unappliedPatch = assert(
			tryGetObjects(instanceMap, createMockApiContext(), patch)
				:await()
		)

		assert(PatchSet.isEmpty(unappliedPatch), "expected remaining patch to be empty")

		local doge = instanceMap.fromIds["DOGE"]
		assert(doge ~= nil, "no instance with id DOGE")
		expect(doge.Parent).to.equal(folder)
		expect(doge:FindFirstChild("Child")).to.equal(child)
	end)
end
