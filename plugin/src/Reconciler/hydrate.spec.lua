return function()
	local hydrate = require(script.Parent.hydrate)

	local InstanceMap = require(script.Parent.Parent.InstanceMap)

	it("should match the root instance no matter what", function()
		local knownInstances = InstanceMap.new()
		local virtualInstances = {
			ROOT = {
				ClassName = "Model",
				Name = "Foo",
				Properties = {},
				Children = {},
			},
		}

		local rootInstance = Instance.new("Folder")

		hydrate(knownInstances, virtualInstances, "ROOT", rootInstance)

		expect(knownInstances:size()).to.equal(1)
		expect(knownInstances.fromIds["ROOT"]).to.equal(rootInstance)
	end)

	it("should not match children with mismatched ClassName", function()
		local knownInstances = InstanceMap.new()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Root",
				Properties = {},
				Children = {},
			},

			CHILD = {
				ClassName = "Folder",
				Name = "Child",
				Properties = {},
				Children = {},
			},
		}

		local rootInstance = Instance.new("Folder")

		-- ClassName of this instance is intentionally different
		local child = Instance.new("Model")
		child.Name = "Child"
		child.Parent = rootInstance

		hydrate(knownInstances, virtualInstances, "ROOT", rootInstance)

		expect(knownInstances:size()).to.equal(1)
		expect(knownInstances.fromIds["ROOT"]).to.equal(rootInstance)
	end)

	it("should not match children with mismatched Name", function()
		local knownInstances = InstanceMap.new()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Root",
				Properties = {},
				Children = {},
			},

			CHILD = {
				ClassName = "Folder",
				Name = "Child",
				Properties = {},
				Children = {},
			},
		}

		local rootInstance = Instance.new("Folder")

		-- Name of this instance is intentionally different
		local child = Instance.new("Folder")
		child.Name = "Not Child"
		child.Parent = rootInstance

		hydrate(knownInstances, virtualInstances, "ROOT", rootInstance)

		expect(knownInstances:size()).to.equal(1)
		expect(knownInstances.fromIds["ROOT"]).to.equal(rootInstance)
	end)

	it("should pair instances with matching Name and ClassName", function()
		local knownInstances = InstanceMap.new()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Root",
				Properties = {},
				Children = { "CHILD1", "CHILD2" },
			},

			CHILD1 = {
				ClassName = "Folder",
				Name = "Child 1",
				Properties = {},
				Children = {},
			},

			CHILD2 = {
				ClassName = "Model",
				Name = "Child 2",
				Properties = {},
				Children = {},
			},
		}

		local rootInstance = Instance.new("Folder")

		local child1 = Instance.new("Folder")
		child1.Name = "Child 1"
		child1.Parent = rootInstance

		local child2 = Instance.new("Model")
		child2.Name = "Child 2"
		child2.Parent = rootInstance

		hydrate(knownInstances, virtualInstances, "ROOT", rootInstance)

		expect(knownInstances:size()).to.equal(3)
		expect(knownInstances.fromIds["ROOT"]).to.equal(rootInstance)
		expect(knownInstances.fromIds["CHILD1"]).to.equal(child1)
		expect(knownInstances.fromIds["CHILD2"]).to.equal(child2)
	end)
end
