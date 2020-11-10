return function()
	local reify = require(script.Parent.reify)

	local PatchSet = require(script.Parent.Parent.PatchSet)
	local InstanceMap = require(script.Parent.Parent.InstanceMap)
	local Error = require(script.Parent.Error)

	local function isEmpty(table)
		return next(table) == nil, "Table was not empty"
	end

	local function size(dict)
		local len = 0

		for _ in pairs(dict) do
			len = len + 1
		end

		return len
	end

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
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT", nil)

		assert(instanceMap:size() == 0, "expected instanceMap to be empty")

		expect(size(unappliedPatch.added)).to.equal(1)
		expect(unappliedPatch.added["ROOT"]).to.equal(virtualInstances["ROOT"])

		assert(isEmpty(unappliedPatch.removed), "expected no removes")
		assert(isEmpty(unappliedPatch.updated), "expected no updates")
	end)

	it("should assign name and properties", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "StringValue",
				Name = "Spaghetti",
				Properties = {
					Value = {
						Type = "String",
						Value = "Hello, world!",
					},
				},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		assert(PatchSet.isEmpty(unappliedPatch), "expected remaining patch to be empty")

		local instance = instanceMap.fromIds["ROOT"]
		expect(instance.ClassName).to.equal("StringValue")
		expect(instance.Name).to.equal("Spaghetti")
		expect(instance.Value).to.equal("Hello, world!")

		expect(instanceMap:size()).to.equal(1)
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
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		assert(PatchSet.isEmpty(unappliedPatch), "expected remaining patch to be empty")

		local instance = instanceMap.fromIds["ROOT"]
		expect(instance.ClassName).to.equal("Folder")
		expect(instance.Name).to.equal("Parent")

		local child = instance.Child
		expect(child.ClassName).to.equal("Folder")

		expect(instanceMap:size()).to.equal(2)
	end)

	it("should still construct parents if children fail", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Parent",
				Properties = {},
				Children = {"CHILD"},
			},

			CHILD = {
				ClassName = "this ain't an Instance",
				Name = "Child",
				Properties = {},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		expect(size(unappliedPatch.added)).to.equal(1)
		expect(unappliedPatch.added["CHILD"]).to.equal(virtualInstances["CHILD"])
		assert(isEmpty(unappliedPatch.updated), "expected no updates")
		assert(isEmpty(unappliedPatch.removed), "expected no removes")

		local instance = instanceMap.fromIds["ROOT"]
		expect(instance.ClassName).to.equal("Folder")
		expect(instance.Name).to.equal("Parent")

		expect(#instance:GetChildren()).to.equal(0)
		expect(instanceMap:size()).to.equal(1)
	end)

	it("should fail gracefully when setting erroneous properties", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "StringValue",
				Name = "Root",
				Properties = {
					Value = {
						Type = "Vector3",
						Value = {1, 2, 3},
					},
				},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		local instance = instanceMap.fromIds["ROOT"]
		expect(instance.ClassName).to.equal("StringValue")
		expect(instance.Name).to.equal("Root")

		assert(isEmpty(unappliedPatch.added), "expected no additions")
		expect(#unappliedPatch.updated).to.equal(1)
		assert(isEmpty(unappliedPatch.removed), "expected no removes")

		local update = unappliedPatch.updated[1]
		expect(update.id).to.equal("ROOT")
		expect(size(update.changedProperties)).to.equal(1)

		local property = update.changedProperties["Value"]
		expect(property).to.equal(virtualInstances["ROOT"].Properties.Value)
	end)
end