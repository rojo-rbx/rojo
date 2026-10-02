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

	it("should disambiguate duplicate-named siblings by matching properties", function()
		local knownInstances = InstanceMap.new()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Root",
				Properties = {},
				Children = { "CHILD_A", "CHILD_B" },
			},

			CHILD_A = {
				ClassName = "StringValue",
				Name = "a",
				Properties = { Value = { String = "first" } },
				Children = {},
			},

			CHILD_B = {
				ClassName = "StringValue",
				Name = "a",
				Properties = { Value = { String = "second" } },
				Children = {},
			},
		}

		local rootInstance = Instance.new("Folder")

		-- Created in the reverse order of the virtual children, so a purely
		-- order-based tiebreak would mis-pair them.
		local child1 = Instance.new("StringValue")
		child1.Name = "a"
		child1.Value = "second"
		child1.Parent = rootInstance

		local child2 = Instance.new("StringValue")
		child2.Name = "a"
		child2.Value = "first"
		child2.Parent = rootInstance

		hydrate(knownInstances, virtualInstances, "ROOT", rootInstance)

		expect(knownInstances:size()).to.equal(3)
		expect(knownInstances.fromIds["CHILD_A"]).to.equal(child2)
		expect(knownInstances.fromIds["CHILD_B"]).to.equal(child1)
	end)

	it("should fall back to child order for duplicate-named siblings with no distinguishing properties", function()
		local knownInstances = InstanceMap.new()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Root",
				Properties = {},
				Children = { "CHILD_A", "CHILD_B" },
			},

			CHILD_A = {
				ClassName = "Folder",
				Name = "a",
				Properties = {},
				Children = {},
			},

			CHILD_B = {
				ClassName = "Folder",
				Name = "a",
				Properties = {},
				Children = {},
			},
		}

		local rootInstance = Instance.new("Folder")

		local child1 = Instance.new("Folder")
		child1.Name = "a"
		child1.Parent = rootInstance

		local child2 = Instance.new("Folder")
		child2.Name = "a"
		child2.Parent = rootInstance

		hydrate(knownInstances, virtualInstances, "ROOT", rootInstance)

		expect(knownInstances:size()).to.equal(3)
		-- With equal scores the earliest unvisited child wins, preserving the
		-- original order-based behavior.
		expect(knownInstances.fromIds["CHILD_A"]).to.equal(child1)
		expect(knownInstances.fromIds["CHILD_B"]).to.equal(child2)
	end)

	it("should fall back to child order for very large duplicate-named groups", function()
		-- More candidates than hydrate is willing to score at once. The group
		-- must fall back to order-based matching, so virtual child N pairs with
		-- existing child N regardless of properties.
		local count = 64

		local knownInstances = InstanceMap.new()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Root",
				Properties = {},
				Children = {},
			},
		}

		local rootInstance = Instance.new("Folder")

		local expectedInstances = {}
		for i = 1, count do
			local id = "CHILD_" .. i
			table.insert(virtualInstances.ROOT.Children, id)
			virtualInstances[id] = {
				ClassName = "StringValue",
				Name = "a",
				-- Distinct values that, if scored, would pair by value rather
				-- than by order.
				Properties = { Value = { String = "value " .. i } },
				Children = {},
			}

			local child = Instance.new("StringValue")
			child.Name = "a"
			child.Value = "value " .. (count - i + 1)
			child.Parent = rootInstance
			expectedInstances[id] = child
		end

		hydrate(knownInstances, virtualInstances, "ROOT", rootInstance)

		expect(knownInstances:size()).to.equal(count + 1)
		for id, expectedInstance in expectedInstances do
			expect(knownInstances.fromIds[id]).to.equal(expectedInstance)
		end
	end)
end
