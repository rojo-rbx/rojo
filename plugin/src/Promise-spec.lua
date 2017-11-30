return function()
	local Promise = require(script.Parent.Promise)

	describe("Promise.new", function()
		it("should instantiate with a callback", function()
			local promise = Promise.new(function() end)

			expect(promise).to.be.ok()
		end)

		it("should invoke the given callback with resolve and reject", function()
			local callCount = 0
			local resolveArg
			local rejectArg

			local promise = Promise.new(function(resolve, reject)
				callCount = callCount + 1
				resolveArg = resolve
				rejectArg = reject
			end)

			expect(promise).to.be.ok()

			expect(callCount).to.equal(1)
			expect(resolveArg).to.be.a("function")
			expect(rejectArg).to.be.a("function")
			expect(promise._status).to.equal(Promise.Status.Started)
		end)

		it("should resolve promises on resolve()", function()
			local callCount = 0

			local promise = Promise.new(function(resolve)
				callCount = callCount + 1
				resolve()
			end)

			expect(promise).to.be.ok()
			expect(callCount).to.equal(1)
			expect(promise._status).to.equal(Promise.Status.Resolved)
		end)

		it("should reject promises on reject()", function()
			local callCount = 0

			local promise = Promise.new(function(resolve, reject)
				callCount = callCount + 1
				reject()
			end)

			expect(promise).to.be.ok()
			expect(callCount).to.equal(1)
			expect(promise._status).to.equal(Promise.Status.Rejected)
		end)

		it("should reject on error in callback", function()
			local callCount = 0

			local promise = Promise.new(function()
				callCount = callCount + 1
				error("hahah")
			end)

			expect(promise).to.be.ok()
			expect(callCount).to.equal(1)
			expect(promise._status).to.equal(Promise.Status.Rejected)
			expect(promise._value[1]:find("hahah")).to.be.ok()
		end)
	end)
end
