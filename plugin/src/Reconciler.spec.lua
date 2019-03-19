local Reconciler = require(script.Parent.Reconciler)

return function()
	it("should leave instances alone if there's nothing specified", function()
		local instance = Instance.new("Folder")
		instance.Name = "TestFolder"

		local instanceId = "test-id"
		local virtualInstancesById = {
			[instanceId] = {
				Name = "TestFolder",
				ClassName = "Folder",
				Children = {},
				Properties = {},
			},
		}

		local reconciler = Reconciler.new()
		reconciler:reconcile(virtualInstancesById, instanceId, instance)
	end)

	it("should assign names from virtual instances", function()
		local instance = Instance.new("Folder")
		instance.Name = "InitialName"

		local instanceId = "test-id"
		local virtualInstancesById = {
			[instanceId] = {
				Name = "NewName",
				ClassName = "Folder",
				Children = {},
				Properties = {},
			},
		}

		local reconciler = Reconciler.new()
		reconciler:reconcile(virtualInstancesById, instanceId, instance)

		expect(instance.Name).to.equal("NewName")
	end)

	it("should assign properties from virtual instances", function()
		local instance = Instance.new("IntValue")
		instance.Name = "TestValue"
		instance.Value = 5

		local instanceId = "test-id"
		local virtualInstancesById = {
			[instanceId] = {
				Name = "TestValue",
				ClassName = "IntValue",
				Children = {},
				Properties = {
					Value = {
						Type = "Int32",
						Value = 9
					}
				},
			},
		}

		local reconciler = Reconciler.new()
		reconciler:reconcile(virtualInstancesById, instanceId, instance)

		expect(instance.Value).to.equal(9)
	end)

	it("should wipe unknown children by default", function()
		local parent = Instance.new("Folder")
		parent.Name = "Parent"

		local child = Instance.new("Folder")
		child.Name = "Child"

		local parentId = "test-id"
		local virtualInstancesById = {
			[parentId] = {
				Name = "Parent",
				ClassName = "Folder",
				Children = {},
				Properties = {},
			},
		}

		local reconciler = Reconciler.new()
		reconciler:reconcile(virtualInstancesById, parentId, parent)

		expect(#parent:GetChildren()).to.equal(0)
	end)

	it("should preserve unknown children if ignoreUnknownInstances is set", function()
		local parent = Instance.new("Folder")
		parent.Name = "Parent"

		local child = Instance.new("Folder")
		child.Parent = parent
		child.Name = "Child"

		local parentId = "test-id"
		local virtualInstancesById = {
			[parentId] = {
				Name = "Parent",
				ClassName = "Folder",
				Children = {},
				Properties = {},
				Metadata = {
					ignoreUnknownInstances = true,
				},
			},
		}

		local reconciler = Reconciler.new()
		reconciler:reconcile(virtualInstancesById, parentId, parent)

		expect(child.Parent).to.equal(parent)
		expect(#parent:GetChildren()).to.equal(1)
	end)

	it("should remove known removed children", function()
		local parent = Instance.new("Folder")
		parent.Name = "Parent"

		local child = Instance.new("Folder")
		child.Parent = parent
		child.Name = "Child"

		local parentId = "parent-id"
		local childId = "child-id"

		local reconciler = Reconciler.new()

		local virtualInstancesById = {
			[parentId] = {
				Name = "Parent",
				ClassName = "Folder",
				Children = {childId},
				Properties = {},
			},
			[childId] = {
				Name = "Child",
				ClassName = "Folder",
				Children = {},
				Properties = {},
			},
		}
		reconciler:reconcile(virtualInstancesById, parentId, parent)

		expect(child.Parent).to.equal(parent)
		expect(#parent:GetChildren()).to.equal(1)

		local newVirtualInstances = {
			[parentId] = {
				Name = "Parent",
				ClassName = "Folder",
				Children = {},
				Properties = {},
			},
			[childId] = nil,
		}
		reconciler:reconcile(newVirtualInstances, parentId, parent)

		expect(child.Parent).to.equal(nil)
		expect(#parent:GetChildren()).to.equal(0)
	end)

	it("should remove known removed children if ignoreUnknownInstances is set", function()
		local parent = Instance.new("Folder")
		parent.Name = "Parent"

		local child = Instance.new("Folder")
		child.Parent = parent
		child.Name = "Child"

		local parentId = "parent-id"
		local childId = "child-id"

		local reconciler = Reconciler.new()

		local virtualInstancesById = {
			[parentId] = {
				Name = "Parent",
				ClassName = "Folder",
				Children = {childId},
				Properties = {},
				Metadata = {
					ignoreUnknownInstances = true,
				},
			},
			[childId] = {
				Name = "Child",
				ClassName = "Folder",
				Children = {},
				Properties = {},
			},
		}
		reconciler:reconcile(virtualInstancesById, parentId, parent)

		expect(child.Parent).to.equal(parent)
		expect(#parent:GetChildren()).to.equal(1)

		local newVirtualInstances = {
			[parentId] = {
				Name = "Parent",
				ClassName = "Folder",
				Children = {},
				Properties = {},
				Metadata = {
					ignoreUnknownInstances = true,
				},
			},
			[childId] = nil,
		}
		reconciler:reconcile(newVirtualInstances, parentId, parent)

		expect(child.Parent).to.equal(nil)
		expect(#parent:GetChildren()).to.equal(0)
	end)
end