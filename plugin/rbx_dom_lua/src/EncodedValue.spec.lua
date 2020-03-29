return function()
	local RbxDom = require(script.Parent)
	local EncodedValue = require(script.Parent.EncodedValue)

	it("should decode Rect values", function()
		local input = {
			Type = "Rect",
			Value = {
				Min = {1, 2},
				Max = {3, 4},
			},
		}

		local output = Rect.new(1, 2, 3, 4)

		local ok, decoded = EncodedValue.decode(input)

		assert(ok, decoded)
		expect(decoded).to.equal(output)
	end)

	it("should decode ColorSequence values", function()
		local input = {
			Type = "ColorSequence",
			Value = {
				Keypoints = {
					{
						Time = 0,
						Color = { 0.12, 0.34, 0.56 },
					},

					{
						Time = 1,
						Color = { 0.13, 0.33, 0.37 },
					},
				}
			},
		}

		local output = ColorSequence.new({
			ColorSequenceKeypoint.new(0, Color3.new(0.12, 0.34, 0.56)),
			ColorSequenceKeypoint.new(1, Color3.new(0.13, 0.33, 0.37)),
		})

		local ok, decoded = EncodedValue.decode(input)
		assert(ok, decoded)
		expect(decoded).to.equal(output)
	end)

	it("should decode NumberSequence values", function()
		local input = {
			Type = "NumberSequence",
			Value = {
				Keypoints = {
					{
						Time = 0,
						Value = 0.5,
						Envelope = 0,
					},

					{
						Time = 1,
						Value = 0.5,
						Envelope = 0,
					},
				}
			},
		}

		local output = NumberSequence.new({
			NumberSequenceKeypoint.new(0, 0.5, 0),
			NumberSequenceKeypoint.new(1, 0.5, 0),
		})

		local ok, decoded = EncodedValue.decode(input)
		assert(ok, decoded)
		expect(decoded).to.equal(output)
	end)

	it("should decode PhysicalProperties values", function()
		local input = {
			Type = "PhysicalProperties",
			Value = {
				Density = 0.1,
				Friction = 0.2,
				Elasticity = 0.3,
				FrictionWeight = 0.4,
				ElasticityWeight = 0.5,
			},
		}

		local output = PhysicalProperties.new(
			0.1,
			0.2,
			0.3,
			0.4,
			0.5
		)

		local ok, decoded = EncodedValue.decode(input)
		assert(ok, decoded)
		expect(decoded).to.equal(output)
	end)

	-- This part of rbx_dom_lua needs some work still.
	itSKIP("should encode Rect values", function()
		local input = Rect.new(10, 20, 30, 40)

		local output = {
			Type = "Rect",
			Value = {
				Min = {10, 20},
				Max = {30, 40},
			},
		}

		local descriptor = RbxDom.findCanonicalPropertyDescriptor("ImageLabel", "SliceCenter")
		local ok, encoded = EncodedValue.encode(input, descriptor)

		assert(ok, encoded)
		expect(encoded.Type).to.equal(output.Type)
		expect(encoded.Value.Min[1]).to.equal(output.Value.Min[1])
		expect(encoded.Value.Min[2]).to.equal(output.Value.Min[2])
		expect(encoded.Value.Max[1]).to.equal(output.Value.Max[1])
		expect(encoded.Value.Max[2]).to.equal(output.Value.Max[2])
	end)
end