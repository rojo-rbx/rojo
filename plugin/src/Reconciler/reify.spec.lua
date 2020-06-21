return function()
	local reify = require(script.Parent.reify)

	local Error = require(script.Parent.Error)

	it("should throw when given a bogus ID", function()
		expect(function()
			reify({}, "Hi, mom!", game)
		end).to.throw()
	end)

	it("should return an error when given bogus class names", function()
		local virtualInstanceMap = {
			ROOT = {
				ClassName = "Balogna"
			}
		}

		local ok, err = reify(virtualInstanceMap, "ROOT", nil)

		expect(ok).to.equal(false)
		expect(err.kind).to.equal(Error.CannotCreateInstance)
	end)

	it("should assign name and properties", function()
		local virtualInstanceMap = {
			ROOT = {
				ClassName = "StringValue",
				Name = "Spaghetti",
				Properties = {
					Value = {
						Type = "String",
						Value = "Hello, world!"
					}
				}
			}
		}

		local ok, instance = reify(virtualInstanceMap, "ROOT")

		if not ok then
			error(tostring(instance))
		end

		expect(instance.ClassName).to.equal("StringValue")
		expect(instance.Name).to.equal("Spaghetti")
		expect(instance.Value).to.equal("Hello, world!")
	end)
end