return function()
	local countMatchingProperties = require(script.Parent.countMatchingProperties)

	local InstanceMap = require(script.Parent.Parent.InstanceMap)

	it("counts properties whose values match the instance", function()
		local instance = Instance.new("StringValue")
		instance.Value = "hello"

		local virtualInstance = {
			ClassName = "StringValue",
			Name = "Value",
			Properties = {
				Value = { String = "hello" },
			},
			Children = {},
		}

		expect(countMatchingProperties(instance, virtualInstance, InstanceMap.new())).to.equal(1)
	end)

	it("does not count properties whose values differ", function()
		local instance = Instance.new("StringValue")
		instance.Value = "hello"

		local virtualInstance = {
			ClassName = "StringValue",
			Name = "Value",
			Properties = {
				Value = { String = "different" },
			},
			Children = {},
		}

		expect(countMatchingProperties(instance, virtualInstance, InstanceMap.new())).to.equal(0)
	end)

	it("counts multiple matching properties independently", function()
		local instance = Instance.new("Part")
		instance.Anchored = true
		instance.CanCollide = false

		local virtualInstance = {
			ClassName = "Part",
			Name = "Part",
			Properties = {
				Anchored = { Bool = true },
				CanCollide = { Bool = false },
			},
			Children = {},
		}

		expect(countMatchingProperties(instance, virtualInstance, InstanceMap.new())).to.equal(2)

		-- Flip one so only a single property matches.
		instance.CanCollide = true
		expect(countMatchingProperties(instance, virtualInstance, InstanceMap.new())).to.equal(1)
	end)

	it("skips unknown properties without counting or erroring", function()
		local instance = Instance.new("Folder")

		local virtualInstance = {
			ClassName = "Folder",
			Name = "Folder",
			Properties = {
				FAKE_PROPERTY = { String = "nope" },
			},
			Children = {},
		}

		expect(countMatchingProperties(instance, virtualInstance, InstanceMap.new())).to.equal(0)
	end)

	it("skips Ref properties without counting or erroring", function()
		local instance = Instance.new("ObjectValue")

		local virtualInstance = {
			ClassName = "ObjectValue",
			Name = "ObjectValue",
			Properties = {
				-- A ref must be skipped rather than decoded: during hydration
				-- the target may not be in the map yet.
				Value = { Ref = "00000000000000000000000000000000" },
			},
			Children = {},
		}

		expect(countMatchingProperties(instance, virtualInstance, InstanceMap.new())).to.equal(0)
	end)
end
