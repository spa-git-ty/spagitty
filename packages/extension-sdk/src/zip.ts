// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The ZIP container a `.spagitty-extension` is, written and read with
 * `node:zlib` alone: deflate for the bytes, a CRC-32 table here. Only what a
 * package needs — no ZIP64, no encryption, no comments.
 */

import { deflateRawSync, inflateRawSync } from 'node:zlib';

const TABLE = (() => {
	const table = new Uint32Array(256);
	for (let n = 0; n < 256; n++) {
		let c = n;
		for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
		table[n] = c >>> 0;
	}
	return table;
})();

export function crc32(data: Uint8Array): number {
	let crc = 0xffffffff;
	for (let i = 0; i < data.length; i++) crc = TABLE[(crc ^ data[i]) & 0xff] ^ (crc >>> 8);
	return (crc ^ 0xffffffff) >>> 0;
}

export interface ZipEntry {
	name: string;
	data: Uint8Array;
	/** A Unix mode such as 0o100755. */
	mode: number;
}

export function writeZip(entries: ZipEntry[]): Uint8Array {
	const chunks: Buffer[] = [];
	const central: Buffer[] = [];
	let offset = 0;
	for (const entry of entries) {
		const name = Buffer.from(entry.name, 'utf8');
		const compressed = deflateRawSync(entry.data);
		const crc = crc32(entry.data);
		const local = Buffer.alloc(30);
		local.writeUInt32LE(0x04034b50, 0);
		local.writeUInt16LE(20, 4);
		local.writeUInt16LE(0x0800, 6);
		local.writeUInt16LE(8, 8);
		local.writeUInt32LE(0x00210000, 10);
		local.writeUInt32LE(crc, 14);
		local.writeUInt32LE(compressed.length, 18);
		local.writeUInt32LE(entry.data.length, 22);
		local.writeUInt16LE(name.length, 26);
		local.writeUInt16LE(0, 28);
		chunks.push(local, name, compressed);

		const header = Buffer.alloc(46);
		header.writeUInt32LE(0x02014b50, 0);
		header.writeUInt16LE((3 << 8) | 20, 4);
		header.writeUInt16LE(20, 6);
		header.writeUInt16LE(0x0800, 8);
		header.writeUInt16LE(8, 10);
		header.writeUInt32LE(0x00210000, 12);
		header.writeUInt32LE(crc, 16);
		header.writeUInt32LE(compressed.length, 20);
		header.writeUInt32LE(entry.data.length, 24);
		header.writeUInt16LE(name.length, 28);
		header.writeUInt32LE((entry.mode << 16) >>> 0, 38);
		header.writeUInt32LE(offset, 42);
		central.push(header, name);
		offset += local.length + name.length + compressed.length;
	}
	const directory = Buffer.concat(central);
	const end = Buffer.alloc(22);
	end.writeUInt32LE(0x06054b50, 0);
	end.writeUInt16LE(entries.length, 8);
	end.writeUInt16LE(entries.length, 10);
	end.writeUInt32LE(directory.length, 12);
	end.writeUInt32LE(offset, 16);
	return new Uint8Array(Buffer.concat([...chunks, directory, end]));
}

/** Read every entry. Trusts nothing it cannot check: sizes and CRCs are verified. */
export function readZip(bytes: Uint8Array): ZipEntry[] {
	const buffer = Buffer.from(bytes);
	let eocd = -1;
	for (let i = buffer.length - 22; i >= Math.max(0, buffer.length - 65557); i--) {
		if (buffer.readUInt32LE(i) === 0x06054b50) {
			eocd = i;
			break;
		}
	}
	if (eocd < 0) throw new Error('not a ZIP archive');
	const count = buffer.readUInt16LE(eocd + 10);
	let at = buffer.readUInt32LE(eocd + 16);
	const out: ZipEntry[] = [];
	for (let i = 0; i < count; i++) {
		if (buffer.readUInt32LE(at) !== 0x02014b50) throw new Error('damaged directory');
		const method = buffer.readUInt16LE(at + 10);
		const crc = buffer.readUInt32LE(at + 16);
		const compressedSize = buffer.readUInt32LE(at + 20);
		const size = buffer.readUInt32LE(at + 24);
		const nameLength = buffer.readUInt16LE(at + 28);
		const extra = buffer.readUInt16LE(at + 30);
		const comment = buffer.readUInt16LE(at + 32);
		const external = buffer.readUInt32LE(at + 38);
		const local = buffer.readUInt32LE(at + 42);
		const name = buffer.toString('utf8', at + 46, at + 46 + nameLength);
		at += 46 + nameLength + extra + comment;
		const start = local + 30 + buffer.readUInt16LE(local + 26) + buffer.readUInt16LE(local + 28);
		const raw = buffer.subarray(start, start + compressedSize);
		const data = method === 0 ? raw : method === 8 ? inflateRawSync(raw) : null;
		if (!data) throw new Error(`${name}: unsupported compression`);
		if (data.length !== size || crc32(data) !== crc) throw new Error(`${name}: damaged`);
		if (!name.endsWith('/')) out.push({ name, data: new Uint8Array(data), mode: external >>> 16 });
	}
	return out;
}
