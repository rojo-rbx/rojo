return function()
	local applyPatch = require(script.Parent.applyPatch)

	local InstanceMap = require(script.Parent.Parent.InstanceMap)
	local PatchSet = require(script.Parent.Parent.PatchSet)

	local container = Instance.new("Folder")

	local tempContainer = Instance.new("Folder")
	local function wasRemoved(instance)
		-- If an instance was destroyed, its parent property is locked.
		-- If an instance was removed, its parent property is nil.
		-- We need to ensure we only remove, so that ChangeHistoryService can still Undo.

		local isParentUnlocked = pcall(function()
			local oldParent = instance.Parent
			instance.Parent = tempContainer
			instance.Parent = oldParent
		end)

		return instance.Parent == nil and isParentUnlocked
	end

	beforeEach(function()
		container:ClearAllChildren()
	end)

	afterAll(function()
		container:Destroy()
		tempContainer:Destroy()
	end)

	it("should return an empty patch if given an empty patch", function()
		local patch = applyPatch(InstanceMap.new(), PatchSet.newEmpty())
		assert(PatchSet.isEmpty(patch), "expected remaining patch to be empty")
	end)

	it("should remove instances listed for remove", function()
		local root = Instance.new("Folder")
		root.Name = "ROOT"
		root.Parent = container

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

		assert(not wasRemoved(root), "expected root to be left alone")
		assert(wasRemoved(child), "expected child to be removed")

		instanceMap:stop()
	end)

	it("should remove IDs listed for remove", function()
		local root = Instance.new("Folder")
		root.Name = "ROOT"
		root.Parent = container

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
		expect(instanceMap:size()).to.equal(1)

		assert(not wasRemoved(root), "expected root to be left alone")
		assert(wasRemoved(child), "expected child to be removed")

		instanceMap:stop()
	end)

	it("should add instances to the DOM", function()
		-- Many of the details of this functionality are instead covered by
		-- tests on reify, not here.

		local root = Instance.new("Folder")
		root.Name = "ROOT"
		root.Parent = container

		local instanceMap = InstanceMap.new()
		instanceMap:insert("ROOT", root)

		local patch = PatchSet.newEmpty()
		patch.added["CHILD"] = {
			Id = "CHILD",
			ClassName = "Model",
			Name = "Child",
			Parent = "ROOT",
			Children = { "GRANDCHILD" },
			Properties = {},
		}

		patch.added["GRANDCHILD"] = {
			Id = "GRANDCHILD",
			ClassName = "Part",
			Name = "Grandchild",
			Parent = "CHILD",
			Children = {},
			Properties = {},
		}

		local unapplied = applyPatch(instanceMap, patch)
		assert(PatchSet.isEmpty(unapplied), "expected remaining patch to be empty")
		expect(instanceMap:size()).to.equal(3)

		local child = root:FindFirstChild("Child")
		expect(child).to.be.ok()
		expect(child.ClassName).to.equal("Model")
		expect(child).to.equal(instanceMap.fromIds["CHILD"])

		local grandchild = child:FindFirstChild("Grandchild")
		expect(grandchild).to.be.ok()
		expect(grandchild.ClassName).to.equal("Part")
		expect(grandchild).to.equal(instanceMap.fromIds["GRANDCHILD"])
	end)

	it("should return unapplied additions when instances cannot be created", function()
		local root = Instance.new("Folder")
		root.Name = "ROOT"
		root.Parent = container

		local instanceMap = InstanceMap.new()
		instanceMap:insert("ROOT", root)

		local patch = PatchSet.newEmpty()
		patch.added["OOPSIE"] = {
			Id = "OOPSIE",
			-- Hopefully Roblox never makes an instance with this ClassName.
			ClassName = "UH OH",
			Name = "FUBAR",
			Parent = "ROOT",
			Children = {},
			Properties = {},
		}

		local unapplied = applyPatch(instanceMap, patch)
		expect(unapplied.added["OOPSIE"]).to.equal(patch.added["OOPSIE"])
		expect(instanceMap:size()).to.equal(1)
		expect(#root:GetChildren()).to.equal(0)
	end)

	it("should apply property changes to instances", function()
		local value = Instance.new("StringValue")
		value.Value = "HELLO"

		local instanceMap = InstanceMap.new()
		instanceMap:insert("VALUE", value)

		local patch = PatchSet.newEmpty()
		table.insert(patch.updated, {
			id = "VALUE",
			changedProperties = {
				Value = {
					String = "WORLD",
				},
			},
		})

		local unapplied = applyPatch(instanceMap, patch)
		assert(PatchSet.isEmpty(unapplied), "expected remaining patch to be empty")
		expect(value.Value).to.equal("WORLD")
	end)

	it("should recreate instances when changedClassName is set, preserving children", function()
		local root = Instance.new("Folder")
		root.Name = "Initial Root Name"
		root.Parent = container

		local child = Instance.new("Folder")
		child.Name = "Child"
		child.Parent = root

		local instanceMap = InstanceMap.new()
		instanceMap:insert("ROOT", root)
		instanceMap:insert("CHILD", child)

		local patch = PatchSet.newEmpty()
		table.insert(patch.updated, {
			id = "ROOT",
			changedName = "Updated Root Name",
			changedClassName = "StringValue",
			changedProperties = {
				Value = {
					String = "I am Root",
				},
			},
		})

		local unapplied = applyPatch(instanceMap, patch)
		assert(PatchSet.isEmpty(unapplied), "expected remaining patch to be empty")

		local newRoot = instanceMap.fromIds["ROOT"]
		assert(newRoot ~= root, "expected instance to be recreated")
		expect(newRoot.ClassName).to.equal("StringValue")
		expect(newRoot.Name).to.equal("Updated Root Name")
		expect(newRoot.Value).to.equal("I am Root")

		local newChild = newRoot:FindFirstChild("Child")
		assert(newChild ~= nil, "expected child to be present")
		assert(newChild == child, "expected child to be preserved")
	end)
end
