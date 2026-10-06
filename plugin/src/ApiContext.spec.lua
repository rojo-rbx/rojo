return function()
	local ApiContext = require(script.Parent.ApiContext)

	local formatWebSocketError = ApiContext.__formatWebSocketError
	local lostConnectionMessage = "Lost connection to the Rojo server. Make sure `rojo serve` is still running."

	describe("__formatWebSocketError", function()
		it("should return a friendly message for failed WebSocket receives", function()
			local message = 'Failed ws recv - err: 0 "No error", curlErrBuf: ""'

			expect(formatWebSocketError(400, message)).to.equal(lostConnectionMessage)
		end)

		it("should return a friendly message for peer receive failures", function()
			local message = 'err: 56 "Failure when receiving data from the peer"'

			expect(formatWebSocketError(400, message)).to.equal(lostConnectionMessage)
		end)

		it("should return a friendly message for connection resets", function()
			local message = 'Failed ws recv - err: 54 "Connection reset by peer", curlErrBuf: ""'

			expect(formatWebSocketError(400, message)).to.equal(lostConnectionMessage)
		end)

		it("should preserve unexpected WebSocket errors", function()
			expect(formatWebSocketError(500, "Something else failed")).to.equal(
				"WebSocket error: 500 - Something else failed"
			)
		end)

		it("should handle empty or missing messages", function()
			expect(formatWebSocketError(500, "")).to.equal("WebSocket error: 500 - ")
			expect(formatWebSocketError(500, nil)).to.equal("WebSocket error: 500 - ")
		end)

		it("should always return a string", function()
			expect(typeof(formatWebSocketError(400, "connection reset by peer"))).to.equal("string")
			expect(typeof(formatWebSocketError(500, nil))).to.equal("string")
		end)
	end)
end
