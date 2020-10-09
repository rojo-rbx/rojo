return function()
	local applyPatch = require(script.Parent.applyPatch)

	local InstanceMap = require(script.Parent.Parent.InstanceMap)
	local PatchSet = require(script.Parent.Parent.PatchSet)

	local dummy = Instance.new("Folder")
	local function wasDestroyed(instance)
		-- If an instance was destroyed, its parent property is locked.
		local ok = pcall(function()
			local oldParent = instance.Parent
			instance.Parent = dummy
			instance.Parent = oldParent
		end)

		return not ok
	end

	it("should return an empty patch if given an empty patch", function()
		local patch = applyPatch(InstanceMap.new(), PatchSet.newEmpty())
		assert(PatchSet.isEmpty(patch), "expected remaining patch to be empty")
	end)

	it("should destroy instances listed for remove", function()
		local root = Instance.new("Folder")

		local child = Instance.new("Folder")
		child.Name = "Child"
		child.Parent = root

		local instanceMap = InstanceMap.new()
		instanceMap:insert("ROOT", root)
		instanceMap:insert("CHILD", child)

		local patch = PatchSet.newEmpty()
		table.insert(patch.removed, child)

		local unapplied = applyPatch(instanceMap, patch)
		assert(PatchSet.isEmpty(unapplied), "expected remaining patch to be empty")

		assert(not wasDestroyed(root), "expected root to be left alone")
		assert(wasDestroyed(child), "expected child to be destroyed")

		instanceMap:stop()
	end)

	it("should destroy IDs listed for remove", function()
		local root = Instance.new("Folder")

		local child = Instance.new("Folder")
		child.Name = "Child"
		child.Parent = root

		local instanceMap = InstanceMap.new()
		instanceMap:insert("ROOT", root)
		instanceMap:insert("CHILD", child)

		local patch = PatchSet.newEmpty()
		table.insert(patch.removed, "CHILD")

		local unapplied = applyPatch(instanceMap, patch)
		assert(PatchSet.isEmpty(unapplied), "expected remaining patch to be empty")

		assert(not wasDestroyed(root), "expected root to be left alone")
		assert(wasDestroyed(child), "expected child to be destroyed")

		instanceMap:stop()
	end)
end