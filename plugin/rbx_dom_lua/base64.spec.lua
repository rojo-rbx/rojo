return function()
	local base64 = require(script.Parent.base64)

	it("should encode and decode", function()
		local function try(str, expected)
			local encoded = base64.encode(str)
			expect(encoded).to.equal(expected)
			expect(base64.decode(encoded)).to.equal(str)
		end

		try("Man", "TWFu")
		try("Ma", "TWE=")
		try("M", "TQ==")
		try("ManM", "TWFuTQ==")
		try(
			[[Man is distinguished, not only by his reason, but by this ]]
				.. [[singular passion from other animals, which is a lust of the ]]
				.. [[mind, that by a perseverance of delight in the continued and ]]
				.. [[indefatigable generation of knowledge, exceeds the short ]]
				.. [[vehemence of any carnal pleasure.]],
			[[TWFuIGlzIGRpc3Rpbmd1aXNoZWQsIG5vdCBvbmx5IGJ5IGhpcyByZWFzb24sI]]
				.. [[GJ1dCBieSB0aGlzIHNpbmd1bGFyIHBhc3Npb24gZnJvbSBvdGhlciBhbmltYW]]
				.. [[xzLCB3aGljaCBpcyBhIGx1c3Qgb2YgdGhlIG1pbmQsIHRoYXQgYnkgYSBwZXJ]]
				.. [[zZXZlcmFuY2Ugb2YgZGVsaWdodCBpbiB0aGUgY29udGludWVkIGFuZCBpbmRl]]
				.. [[ZmF0aWdhYmxlIGdlbmVyYXRpb24gb2Yga25vd2xlZGdlLCBleGNlZWRzIHRoZ]]
				.. [[SBzaG9ydCB2ZWhlbWVuY2Ugb2YgYW55IGNhcm5hbCBwbGVhc3VyZS4=]]
		)
	end)
end
