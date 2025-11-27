local function hex(str)
  local result = table.create(2*#str - 1)

  for i=1,#str do
    result[2*(i-1)+1] = string.format("%02X", string.byte(str, i))

    if i ~= #str then
      result[2*(i-1)+2] = " "
    end
  end

  return table.concat(result)
end

return function()
  local msgpack = require(script.Parent.msgpack)

  describe("decode", function()
    it("can decode nil value", function()
      local message = "\xC0"
      expect(msgpack.decode(message)).to.equal(nil)
    end)

    it("can decode false value", function()
      local message = "\xC2"
      expect(msgpack.decode(message)).to.equal(false)
    end)

    it("can decode true value", function()
      local message = "\xC3"
      expect(msgpack.decode(message)).to.equal(true)
    end)

    it("can decode positive fixint value", function()
      expect(msgpack.decode("\x0C")).to.equal(12)
      expect(msgpack.decode("\x00")).to.equal(0)
      expect(msgpack.decode("\x7f")).to.equal(127)
    end)

    it("can decode negative fixint value", function()
      expect(msgpack.decode("\xE0")).to.equal(-32)
      expect(msgpack.decode("\xFF")).to.equal(-1)
      expect(msgpack.decode("\xE7")).to.equal(-25)
    end)

    it("can decode uint 8 value", function()
      expect(msgpack.decode("\xCC\x00")).to.equal(0)
      expect(msgpack.decode("\xCC\xFF")).to.equal(255)
      expect(msgpack.decode("\xCC\x0F")).to.equal(15)
    end)

    it("can decode uint 16 value", function()
      expect(msgpack.decode("\xCD\x00\x00")).to.equal(0)
      expect(msgpack.decode("\xCD\xFF\xFF")).to.equal(65535)
      expect(msgpack.decode("\xCD\x00\xFF")).to.equal(255)
    end)

    it("can decode uint 32 value", function()
      expect(msgpack.decode("\xCE\x00\x00\x00\x00")).to.equal(0)
      expect(msgpack.decode("\xCE\xFF\xFF\xFF\xFF")).to.equal(4294967295)
      expect(msgpack.decode("\xCE\x00\x00\xFF\xFF")).to.equal(65535)
    end)

    it("can decode uint 64 value", function()
      local zeroValue = msgpack.decode("\xCF\x00\x00\x00\x00\x00\x00\x00\x00")
      expect(zeroValue._msgpackType).to.equal(msgpack.UInt64)
      expect(zeroValue.mostSignificantPart).to.equal(0)
      expect(zeroValue.leastSignificantPart).to.equal(0)

      local maxValue = msgpack.decode("\xCF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF")
      expect(maxValue.mostSignificantPart).to.equal(4294967295)
      expect(maxValue.leastSignificantPart).to.equal(4294967295)

      local midValue = msgpack.decode("\xCF\x00\x00\x00\x00\xFF\xFF\xFF\xFF")
      expect(midValue.mostSignificantPart).to.equal(0)
      expect(midValue.leastSignificantPart).to.equal(4294967295)
    end)

    it("can decode int 8 value", function()
      expect(msgpack.decode("\xD0\x00")).to.equal(0)
      expect(msgpack.decode("\xD0\xFF")).to.equal(-1)
      expect(msgpack.decode("\xD0\x0F")).to.equal(15)
      expect(msgpack.decode("\xD0\x7F")).to.equal(127)
      expect(msgpack.decode("\xD0\x80")).to.equal(-128)
    end)

    it("can decode int 16 value", function()
      expect(msgpack.decode("\xD1\x00\x00")).to.equal(0)
      expect(msgpack.decode("\xD1\xFF\xFF")).to.equal(-1)
      expect(msgpack.decode("\xD1\x00\xFF")).to.equal(255)
      expect(msgpack.decode("\xD1\x7F\xFF")).to.equal(32767)
      expect(msgpack.decode("\xD1\x80\x00")).to.equal(-32768)
    end)

    it("can decode int 32 value", function()
      expect(msgpack.decode("\xD2\x00\x00\x00\x00")).to.equal(0)
      expect(msgpack.decode("\xD2\xFF\xFF\xFF\xFF")).to.equal(-1)
      expect(msgpack.decode("\xD2\x00\x00\xFF\xFF")).to.equal(65535)
      expect(msgpack.decode("\xD2\x7F\xFF\xFF\xFF")).to.equal(2147483647)
      expect(msgpack.decode("\xD2\x80\x00\x00\x00")).to.equal(-2147483648)
    end)

    it("can decode int 64 value", function()
      local zeroValue = msgpack.decode("\xD3\x00\x00\x00\x00\x00\x00\x00\x00")
      expect(zeroValue._msgpackType).to.equal(msgpack.Int64)
      expect(zeroValue.mostSignificantPart).to.equal(0)
      expect(zeroValue.leastSignificantPart).to.equal(0)

      local maxValue = msgpack.decode("\xD3\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF")
      expect(maxValue.mostSignificantPart).to.equal(4294967295)
      expect(maxValue.leastSignificantPart).to.equal(4294967295)

      local midValue = msgpack.decode("\xD3\x00\x00\x00\x00\xFF\xFF\xFF\xFF")
      expect(midValue.mostSignificantPart).to.equal(0)
      expect(midValue.leastSignificantPart).to.equal(4294967295)
    end)

    it("can decode float 32 value", function()
      expect(msgpack.decode("\xCA\x00\x00\x00\x00")).to.equal(0)
      expect(msgpack.decode("\xCA\x40\x00\x00\x00")).to.equal(2)
      expect(msgpack.decode("\xCA\xC0\x00\x00\x00")).to.equal(-2)
      expect(msgpack.decode("\xCA\x7F\x80\x00\x00")).to.equal(math.huge)
      expect(msgpack.decode("\xCA\xFF\x80\x00\x00")).to.equal(-math.huge)

      local nan = msgpack.decode("\xCA\xFF\x80\x00\x01")
      expect(nan).to.never.equal(nan)
    end)

    it("can decode float 64 value", function()
      expect(msgpack.decode("\xCB\x00\x00\x00\x00\x00\x00\x00\x00")).to.equal(0)
      expect(msgpack.decode("\xCB\x40\x00\x00\x00\x00\x00\x00\x00")).to.equal(2)
      expect(msgpack.decode("\xCB\xC0\x00\x00\x00\x00\x00\x00\x00")).to.equal(-2)
      expect(msgpack.decode("\xCB\x7F\xF0\x00\x00\x00\x00\x00\x00")).to.equal(math.huge)
      expect(msgpack.decode("\xCB\xFF\xF0\x00\x00\x00\x00\x00\x00")).to.equal(-math.huge)

      local nan = msgpack.decode("\xCB\xFF\xF0\x00\x00\x00\x00\x00\x01")
      expect(nan).to.never.equal(nan)
    end)

    it("can decode fixstr value", function()
      expect(msgpack.decode("\xA0")).to.equal("")
      expect(msgpack.decode("\xA1\x78")).to.equal("x")
      expect(msgpack.decode("\xAB\x68\x65\x6C\x6C\x6F\x20\x77\x6F\x72\x6C\x64")).to.equal("hello world")
      expect(msgpack.decode("\xBF\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61\x61")).to.equal("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    end)

    it("can decode str 8 value", function()
      expect(msgpack.decode("\xD9\x00")).to.equal("")
      expect(msgpack.decode("\xD9\x01\x78")).to.equal("x")
      expect(msgpack.decode("\xD9\x0B\x68\x65\x6C\x6C\x6F\x20\x77\x6F\x72\x6C\x64")).to.equal("hello world")
    end)

    it("can decode str 16 value", function()
      expect(msgpack.decode("\xDA\x00\x00")).to.equal("")
      expect(msgpack.decode("\xDA\x00\x01\x78")).to.equal("x")
      expect(msgpack.decode("\xDA\x00\x0B\x68\x65\x6C\x6C\x6F\x20\x77\x6F\x72\x6C\x64")).to.equal("hello world")
    end)

    it("can decode str 32 value", function()
      expect(msgpack.decode("\xDB\x00\x00\x00\x00")).to.equal("")
      expect(msgpack.decode("\xDB\x00\x00\x00\x01\x78")).to.equal("x")
      expect(msgpack.decode("\xDB\x00\x00\x00\x0B\x68\x65\x6C\x6C\x6F\x20\x77\x6F\x72\x6C\x64")).to.equal("hello world")
    end)

    it("can decode bin 8 value", function()
      local emptyBinary = msgpack.decode("\xC4\x00")
      expect(type(emptyBinary)).to.equal("buffer")
      expect(buffer.tostring(emptyBinary)).to.equal("")

      local xBinary = msgpack.decode("\xC4\x01\x78")
      expect(buffer.tostring(xBinary)).to.equal("x")

      local helloBinary = msgpack.decode("\xC4\x0B\x68\x65\x6C\x6C\x6F\x20\x77\x6F\x72\x6C\x64")
      expect(buffer.tostring(helloBinary)).to.equal("hello world")
    end)

    it("can decode bin 16 value", function()
      local emptyBinary = msgpack.decode("\xC5\x00\x00")
      expect(type(emptyBinary)).to.equal("buffer")
      expect(buffer.tostring(emptyBinary)).to.equal("")

      local xBinary = msgpack.decode("\xC5\x00\x01\x78")
      expect(buffer.tostring(xBinary)).to.equal("x")

      local helloBinary = msgpack.decode("\xC5\x00\x0B\x68\x65\x6C\x6C\x6F\x20\x77\x6F\x72\x6C\x64")
      expect(buffer.tostring(helloBinary)).to.equal("hello world")
    end)

    it("can decode bin 32 value", function()
      local emptyBinary = msgpack.decode("\xC6\x00\x00\x00\x00")
      expect(type(emptyBinary)).to.equal("buffer")
      expect(buffer.tostring(emptyBinary)).to.equal("")

      local xBinary = msgpack.decode("\xC6\x00\x00\x00\x01\x78")
      expect(buffer.tostring(xBinary)).to.equal("x")

      local helloBinary = msgpack.decode("\xC6\x00\x00\x00\x0B\x68\x65\x6C\x6C\x6F\x20\x77\x6F\x72\x6C\x64")
      expect(buffer.tostring(helloBinary)).to.equal("hello world")
    end)

    it("can decode fixarray value", function()
      local emptyArray = msgpack.decode("\x90")
      expect(emptyArray).to.be.a("table")
      expect(#emptyArray).to.equal(0)
      expect(next(emptyArray)).never.to.be.ok()

      local filledArray = msgpack.decode("\x93\xC0\xC2\xC3")
      expect(#filledArray).to.equal(3)
      expect(filledArray[1]).to.equal(nil)
      expect(filledArray[2]).to.equal(false)
      expect(filledArray[3]).to.equal(true)

      local arrayWithString = msgpack.decode("\x93\xC2\xA5\x68\x65\x6C\x6C\x6F\xC3")
      expect(#arrayWithString).to.equal(3)
      expect(arrayWithString[1]).to.equal(false)
      expect(arrayWithString[2]).to.equal("hello")
      expect(arrayWithString[3]).to.equal(true)
    end)

    it("can decode array 16 value", function()
      local emptyArray = msgpack.decode("\xDC\x00\x00")
      expect(emptyArray).to.be.a("table")
      expect(#emptyArray).to.equal(0)
      expect(next(emptyArray)).never.to.be.ok()

      local filledArray = msgpack.decode("\xDC\x00\x03\xC0\xC2\xC3")
      expect(#filledArray).to.equal(3)
      expect(filledArray[1]).to.equal(nil)
      expect(filledArray[2]).to.equal(false)
      expect(filledArray[3]).to.equal(true)

      local arrayWithString = msgpack.decode("\xDC\x00\x03\xC2\xA5\x68\x65\x6C\x6C\x6F\xC3")
      expect(#arrayWithString).to.equal(3)
      expect(arrayWithString[1]).to.equal(false)
      expect(arrayWithString[2]).to.equal("hello")
      expect(arrayWithString[3]).to.equal(true)
    end)

    it("can decode array 32 value", function()
      local emptyArray = msgpack.decode("\xDD\x00\x00\x00\x00")
      expect(emptyArray).to.be.a("table")
      expect(#emptyArray).to.equal(0)
      expect(next(emptyArray)).never.to.be.ok()

      local filledArray = msgpack.decode("\xDD\x00\x00\x00\x03\xC0\xC2\xC3")
      expect(#filledArray).to.equal(3)
      expect(filledArray[1]).to.equal(nil)
      expect(filledArray[2]).to.equal(false)
      expect(filledArray[3]).to.equal(true)

      local arrayWithString = msgpack.decode("\xDD\x00\x00\x00\x03\xC2\xA5\x68\x65\x6C\x6C\x6F\xC3")
      expect(#arrayWithString).to.equal(3)
      expect(arrayWithString[1]).to.equal(false)
      expect(arrayWithString[2]).to.equal("hello")
      expect(arrayWithString[3]).to.equal(true)
    end)

    it("can decode fixmap value", function()
      local emptyMap = msgpack.decode("\x80")
      expect(emptyMap).to.be.a("table")
      expect(#emptyMap).to.equal(0)
      expect(next(emptyMap)).never.to.be.ok()

      local filledMap = msgpack.decode("\x82\xA5\x68\x65\x6C\x6C\x6F\xA5\x77\x6F\x72\x6C\x64\x7B\xC3")
      expect(#filledMap).to.equal(0)
      expect(next(filledMap)).to.be.ok()
      expect(filledMap["hello"]).to.equal("world")
      expect(filledMap[123]).to.equal(true)
    end)

    it("can decode map 16 value", function()
      local emptyMap = msgpack.decode("\xDE\x00\x00")
      expect(emptyMap).to.be.a("table")
      expect(#emptyMap).to.equal(0)
      expect(next(emptyMap)).never.to.be.ok()

      local filledMap = msgpack.decode("\xDE\x00\x02\xA5\x68\x65\x6C\x6C\x6F\xA5\x77\x6F\x72\x6C\x64\x7B\xC3")
      expect(#filledMap).to.equal(0)
      expect(next(filledMap)).to.be.ok()
      expect(filledMap["hello"]).to.equal("world")
      expect(filledMap[123]).to.equal(true)
    end)

    it("can decode map 32 value", function()
      local emptyMap = msgpack.decode("\xDF\x00\x00\x00\x00")
      expect(emptyMap).to.be.a("table")
      expect(#emptyMap).to.equal(0)
      expect(next(emptyMap)).never.to.be.ok()

      local filledMap = msgpack.decode("\xDF\x00\x00\x00\x02\xA5\x68\x65\x6C\x6C\x6F\xA5\x77\x6F\x72\x6C\x64\x7B\xC3")
      expect(#filledMap).to.equal(0)
      expect(next(filledMap)).to.be.ok()
      expect(filledMap["hello"]).to.equal("world")
      expect(filledMap[123]).to.equal(true)
    end)

    it("can decode fixext 1 value", function()
      local extension = msgpack.decode("\xD4\x7B\x78")
      expect(extension._msgpackType).to.equal(msgpack.Extension)
      expect(extension.type).to.equal(123)
      expect(buffer.tostring(extension.data)).to.equal("x")
    end)

    it("can decode fixext 2 value", function()
      local extension = msgpack.decode("\xD5\x7B\x78\x78")
      expect(extension._msgpackType).to.equal(msgpack.Extension)
      expect(extension.type).to.equal(123)
      expect(buffer.tostring(extension.data)).to.equal("xx")
    end)

    it("can decode fixext 4 value", function()
      local extension = msgpack.decode("\xD6\x7B\x78\x78\x78\x78")
      expect(extension._msgpackType).to.equal(msgpack.Extension)
      expect(extension.type).to.equal(123)
      expect(buffer.tostring(extension.data)).to.equal("xxxx")
    end)

    it("can decode fixext 8 value", function()
      local extension = msgpack.decode("\xD7\x7B\x78\x78\x78\x78\x78\x78\x78\x78")
      expect(extension._msgpackType).to.equal(msgpack.Extension)
      expect(extension.type).to.equal(123)
      expect(buffer.tostring(extension.data)).to.equal("xxxxxxxx")
    end)

    it("can decode fixext 16 value", function()
      local extension = msgpack.decode("\xD8\x7B\x78\x78\x78\x78\x78\x78\x78\x78\x78\x78\x78\x78\x78\x78\x78\x78")
      expect(extension._msgpackType).to.equal(msgpack.Extension)
      expect(extension.type).to.equal(123)
      expect(buffer.tostring(extension.data)).to.equal("xxxxxxxxxxxxxxxx")
    end)

    it("can decode ext 8 value", function()
      local emptyExtension = msgpack.decode("\xC7\x00\x7B")
      expect(emptyExtension._msgpackType).to.equal(msgpack.Extension)
      expect(emptyExtension.type).to.equal(123)
      expect(buffer.tostring(emptyExtension.data)).to.equal("")

      local extension = msgpack.decode("\xC7\x01\x7B\x78")
      expect(extension._msgpackType).to.equal(msgpack.Extension)
      expect(extension.type).to.equal(123)
      expect(buffer.tostring(extension.data)).to.equal("x")
    end)

    it("can decode ext 16 value", function()
      local emptyExtension = msgpack.decode("\xC8\x00\x00\x7B")
      expect(emptyExtension._msgpackType).to.equal(msgpack.Extension)
      expect(emptyExtension.type).to.equal(123)
      expect(buffer.tostring(emptyExtension.data)).to.equal("")

      local extension = msgpack.decode("\xC8\x00\x01\x7B\x78")
      expect(extension._msgpackType).to.equal(msgpack.Extension)
      expect(extension.type).to.equal(123)
      expect(buffer.tostring(extension.data)).to.equal("x")
    end)

    it("can decode ext 32 value", function()
      local emptyExtension = msgpack.decode("\xC9\x00\x00\x00\x00\x7B")
      expect(emptyExtension._msgpackType).to.equal(msgpack.Extension)
      expect(emptyExtension.type).to.equal(123)
      expect(buffer.tostring(emptyExtension.data)).to.equal("")

      local extension = msgpack.decode("\xC9\x00\x00\x00\x01\x7B\x78")
      expect(extension._msgpackType).to.equal(msgpack.Extension)
      expect(extension.type).to.equal(123)
      expect(buffer.tostring(extension.data)).to.equal("x")
    end)
  end)

  describe("encode", function()
    it("can encode nil value", function()
      expect(hex(msgpack.encode(nil))).to.equal("C0")
    end)

    it("can encode false value", function()
      expect(hex(msgpack.encode(false))).to.equal("C2")
    end)

    it("can encode true value", function()
      expect(hex(msgpack.encode(true))).to.equal("C3")
    end)

    it("can encode string value", function()
      expect(hex(msgpack.encode(""))).to.equal("A0")
      expect(hex(msgpack.encode("xyz"))).to.equal("A3 78 79 7A")

      local xs = string.rep("x", 0x20)
      expect(msgpack.encode(xs)).to.equal("\xD9\x20" .. xs)

      xs = string.rep("x", 0x100)
      expect(msgpack.encode(xs)).to.equal("\xDA\x01\x00" .. xs)

      xs = string.rep("x", 0x10000)
      expect(msgpack.encode(xs)).to.equal("\xDB\x00\x01\x00\x00" .. xs)
    end)

    it("can encode positive integers", function()
      expect(hex(msgpack.encode(0))).to.equal("00")
      expect(hex(msgpack.encode(1))).to.equal("01")
      expect(hex(msgpack.encode(127))).to.equal("7F")
      expect(hex(msgpack.encode(128))).to.equal("CC 80")
      expect(hex(msgpack.encode(255))).to.equal("CC FF")
      expect(hex(msgpack.encode(256))).to.equal("CD 01 00")
      expect(hex(msgpack.encode(65535))).to.equal("CD FF FF")
      expect(hex(msgpack.encode(65536))).to.equal("CE 00 01 00 00")
      expect(hex(msgpack.encode(4294967295))).to.equal("CE FF FF FF FF")
    end)

    it("can encode negative integers", function()
      expect(hex(msgpack.encode(-1))).to.equal("FF")
      expect(hex(msgpack.encode(-32))).to.equal("E0")
      expect(hex(msgpack.encode(-33))).to.equal("D0 DF")
      expect(hex(msgpack.encode(-128))).to.equal("D0 80")
      expect(hex(msgpack.encode(-129))).to.equal("D1 FF 7F")
      expect(hex(msgpack.encode(-32768))).to.equal("D1 80 00")
      expect(hex(msgpack.encode(-32769))).to.equal("D2 FF FF 7F FF")
      expect(hex(msgpack.encode(-2147483648))).to.equal("D2 80 00 00 00")
    end)

    it("can encode floating points", function()
      expect(hex(msgpack.encode(0 / 0))).to.equal("CA 7F 80 00 01")
      expect(hex(msgpack.encode(math.huge))).to.equal("CA 7F 80 00 00")
      expect(hex(msgpack.encode(-math.huge))).to.equal("CA FF 80 00 00")
      expect(hex(msgpack.encode(0.1))).to.equal("CB 3F B9 99 99 99 99 99 9A")
      expect(hex(msgpack.encode(-0.1))).to.equal("CB BF B9 99 99 99 99 99 9A")
      expect(hex(msgpack.encode(1234.56789))).to.equal("CB 40 93 4A 45 84 F4 C6 E7")
      expect(hex(msgpack.encode(1.7976931348623157e+308))).to.equal("CB 7F EF FF FF FF FF FF FF")
      expect(hex(msgpack.encode(-1.7976931348623157e+308))).to.equal("CB FF EF FF FF FF FF FF FF")
      expect(hex(msgpack.encode(2.2250738585072014e-308))).to.equal("CB 00 10 00 00 00 00 00 00")
      expect(hex(msgpack.encode(-2.2250738585072014e-308))).to.equal("CB 80 10 00 00 00 00 00 00")
    end)

    it("can encode Int64 and UInt64 representations", function()
      expect(hex(msgpack.encode(msgpack.Int64.new(0xFFFFFFFF, 0xFFFFFFFF)))).to.equal("D3 FF FF FF FF FF FF FF FF")
      expect(hex(msgpack.encode(msgpack.Int64.new(0xFAFBFCFD, 0xFEFDFCFB)))).to.equal("D3 FA FB FC FD FE FD FC FB")
      expect(hex(msgpack.encode(msgpack.UInt64.new(0xFFFFFFFF, 0xFFFFFFFF)))).to.equal("CF FF FF FF FF FF FF FF FF")
      expect(hex(msgpack.encode(msgpack.UInt64.new(0xFAFBFCFD, 0xFEFDFCFB)))).to.equal("CF FA FB FC FD FE FD FC FB")
    end)

    it("can encode buffer value", function()
      expect(hex(msgpack.encode(buffer.fromstring("")))).to.equal("C4 00")
      expect(hex(msgpack.encode(buffer.fromstring("xyz")))).to.equal("C4 03 78 79 7A")

      local xs = string.rep("x", 0x20)
      expect(msgpack.encode(buffer.fromstring(xs))).to.equal("\xC4\x20" .. xs)

      xs = string.rep("x", 0x100)
      expect(msgpack.encode(buffer.fromstring(xs))).to.equal("\xC5\x01\x00" .. xs)

      xs = string.rep("x", 0x10000)
      expect(msgpack.encode(buffer.fromstring(xs))).to.equal("\xC6\x00\x01\x00\x00" .. xs)
    end)

    it("can encode Extension value", function()
      expect(hex(msgpack.encode(msgpack.Extension.new(123, buffer.fromstring(""))))).to.equal("C7 00 7B")
      expect(hex(msgpack.encode(msgpack.Extension.new(123, buffer.fromstring("x"))))).to.equal("D4 7B 78")
      expect(hex(msgpack.encode(msgpack.Extension.new(123, buffer.fromstring("xy"))))).to.equal("D5 7B 78 79")
      expect(hex(msgpack.encode(msgpack.Extension.new(123, buffer.fromstring("wxyz"))))).to.equal("D6 7B 77 78 79 7A")
      expect(hex(msgpack.encode(msgpack.Extension.new(123, buffer.fromstring("wxyzwxyz"))))).to.equal("D7 7B 77 78 79 7A 77 78 79 7A")
      expect(hex(msgpack.encode(msgpack.Extension.new(123, buffer.fromstring("wxyzwxyzwxyzwxyz"))))).to.equal("D8 7B 77 78 79 7A 77 78 79 7A 77 78 79 7A 77 78 79 7A")

      local xs = string.rep("x", 0x20)
      expect(msgpack.encode(msgpack.Extension.new(123, buffer.fromstring(xs)))).to.equal("\xC7\x20\x7B" .. xs)

      xs = string.rep("x", 0x100)
      expect(msgpack.encode(msgpack.Extension.new(123, buffer.fromstring(xs)))).to.equal("\xC8\x01\x00\x7B" .. xs)

      xs = string.rep("x", 0x10000)
      expect(msgpack.encode(msgpack.Extension.new(123, buffer.fromstring(xs)))).to.equal("\xC9\x00\x01\x00\x00\x7B" .. xs)
    end)

    it("can encode array-like tables", function()
      expect(hex(msgpack.encode({}))).to.equal("90")
      expect(hex(msgpack.encode({1,2,3}))).to.equal("93 01 02 03")

      local t,s = table.create(15, 0), string.rep("\x00", 15)
      expect(msgpack.encode(t)).to.equal("\x9F" .. s)

      t,s = table.create(30, 0), string.rep("\x00", 30)
      expect(msgpack.encode(t)).to.equal("\xDC\x00\x1E" .. s)

      t,s = table.create(70000, 0), string.rep("\x00", 70000)
      expect(msgpack.encode(t)).to.equal("\xDD\x00\x01\x11\x70" .. s)
    end)

    it("can encode map-like tables", function()
      expect(hex(msgpack.encode({a=1}))).to.equal("81 A1 61 01")
      expect(hex(msgpack.encode({1, a=1}))).to.equal("82 01 01 A1 61 01")

      local t,s = table.create(15, 0), string.rep("\x00", 15)
      expect(msgpack.encode(t)).to.equal("\x9F" .. s)

      t,s = table.create(30, 0), string.rep("\x00", 30)
      expect(msgpack.encode(t)).to.equal("\xDC\x00\x1E" .. s)

      t,s = table.create(70000, 0), string.rep("\x00", 70000)
      expect(msgpack.encode(t)).to.equal("\xDD\x00\x01\x11\x70" .. s)
    end)

    it("can handle cyclic tables", function()
      local cyclicTable = {}
      cyclicTable.self = cyclicTable
      expect(function()
        msgpack.encode(cyclicTable)
      end).to.throw("Can not serialize cyclic table")
    end)
  end)

  describe("utf8Encode", function()
    it("can encode a binary string as UTF-8", function()
      expect(msgpack.utf8Encode("")).to.equal("")

      local binary = string.char(0b11111111)
      local utf = string.char(0b01111111, 0b01000000)
      expect(hex(msgpack.utf8Encode(binary))).to.equal(hex(utf))

      binary = string.char(0b11110000, 0b00111100, 0b00001111)
      utf = string.char(0b01111000, 0b00001111, 0b00000001, 0b01110000)
      expect(hex(msgpack.utf8Encode(binary))).to.equal(hex(utf))

      binary = string.char(0b11111111):rep(8)
      utf = string.char(0b01111111):rep(9) .. string.char(0b01000000)
      expect(hex(msgpack.utf8Encode(binary))).to.equal(hex(utf))
    end)
  end)

  describe("utf8Decode", function()
    it("can decode a binary string encoded as UTF-8", function()
      expect(msgpack.utf8Decode("")).to.equal("")

      local utf = string.char(0b01111111, 0b01000000)
      local binary = string.char(0b11111111)
      expect(hex(msgpack.utf8Decode(utf))).to.equal(hex(binary))

      utf = string.char(0b01111000, 0b00001111, 0b00000001, 0b01110000)
      binary = string.char(0b11110000, 0b00111100, 0b00001111)
      expect(hex(msgpack.utf8Decode(utf))).to.equal(hex(binary))

      utf = string.char(0b01111111):rep(9) .. string.char(0b01000000)
      binary = string.char(0b11111111):rep(8)
      expect(hex(msgpack.utf8Decode(utf))).to.equal(hex(binary))
    end)
  end)
end
