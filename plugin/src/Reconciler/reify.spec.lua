return function()
	local reify = require(script.Parent.reify)

	local InstanceMap = require(script.Parent.Parent.InstanceMap)
	local Error = require(script.Parent.Error)

	it("should throw when given a bogus ID", function()
		expect(function()
			reify(InstanceMap.new(), {}, "Hi, mom!", game)
		end).to.throw()
	end)

	it("should return an error when given bogus class names", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "Balogna",
				Name = "Food",
				Properties = {},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local ok, err = reify(instanceMap, virtualInstances, "ROOT", nil)

		expect(ok).to.equal(false)
		expect(err.kind).to.equal(Error.CannotCreateInstance)

		assert(instanceMap:size() == 0, "expected instanceMap to be empty")
	end)

	it("should assign name and properties", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "StringValue",
				Name = "Spaghetti",
				Properties = {
					Value = {
						Type = "String",
						Value = "Hello, world!"
					}
				},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local ok, instance = reify(instanceMap, virtualInstances, "ROOT")

		if not ok then
			error(tostring(instance))
		end

		expect(instance.ClassName).to.equal("StringValue")
		expect(instance.Name).to.equal("Spaghetti")
		expect(instance.Value).to.equal("Hello, world!")

		expect(instanceMap:size()).to.equal(1)
		expect(instanceMap.fromIds["ROOT"]).to.equal(instance)
	end)

	it("should construct children", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Parent",
				Properties = {},
				Children = {"CHILD"},
			},

			CHILD = {
				ClassName = "Folder",
				Name = "Child",
				Properties = {},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local ok, instance = reify(instanceMap, virtualInstances, "ROOT")

		if not ok then
			error(tostring(instance))
		end

		expect(instance.ClassName).to.equal("Folder")
		expect(instance.Name).to.equal("Parent")

		local child = instance.Child
		expect(child.ClassName).to.equal("Folder")

		expect(instanceMap:size()).to.equal(2)
		expect(instanceMap.fromIds["ROOT"]).to.equal(instance)
		expect(instanceMap.fromIds["CHILD"]).to.equal(child)
	end)
end