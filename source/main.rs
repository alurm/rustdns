use std::net;
use std::net::SocketAddr;

fn split_off<'a>(data: &'a mut &[u8], n: usize) -> &'a [u8] {
	let (l, r) = (*data).split_at(n);
	*data = r;
	l
}

fn split_off_const<'a, const N: usize>(data: &'a mut &[u8]) -> [u8; N] {
	let l = split_off(data, N);
	<[u8; N]>::try_from(l).unwrap()
}

struct Question {
	id: [u8; 2],
	domain_name: Vec<Vec<u8>>,
}

fn parse(mut request: &[u8]) -> Question {
	let id = split_off_const::<2>(&mut request);

	// // Won't work reliably (?), need to read bits via network order.
	// {
	// 	split_off_const::<2>(&mut request);
	// 	// Query, not response.
	// 	let qr = split_off_const::<1>(&mut request);
	// 	dbg!(qr);

	// 	let opcode = split_off_const::<4>(&mut request);
	// 	dbg!(opcode);

	// 	let _aa = split_off_const::<1>(&mut request);
	// 	dbg!(_aa);

	// 	let tc = split_off_const::<1>(&mut request);
	// 	dbg!(tc);

	// 	let rc = split_off_const::<1>(&mut request);
	// 	dbg!(rc);

	// 	let ra = split_off_const::<1>(&mut request);
	// 	dbg!(ra);

	// 	let z = split_off_const::<z>(&mut request);
	// 	dbg!(z);
	// }

	// https://mislove.org/teaching/cs4700/spring11/handouts/project1-primer.pdf
	// Assumptions:
	// - message is a query,
	// - it is a "standard query",
	// - message is not truncated,
	// - recursion is irrelevant,
	// - reserved field is set to zero,
	// - there is only one question.
	let _ = split_off_const::<{ 2 * 5 }>(&mut request);

	// Not sure if byte order matters here.

	let domain_name = {
		let mut result = Vec::<Vec<u8>>::new();

		loop {
			let [n] = split_off_const::<1>(&mut request);
			if n == 0 { break }
			// Byte order?
			let name = split_off(&mut request, n.into());
			// let name = std::str::from_utf8(name).unwrap();
			result.push(name.to_vec());
		}

		result
	};

	// Assumptions:
	// - qtype is 1 "A", which needs the host's address,
	// - qclass is 1, which means "IP".
	let _qtype = split_off_const::<2>(&mut request);
	let _qclass = split_off_const::<2>(&mut request);

	Question { id: id, domain_name: domain_name }
}

fn reply(source: SocketAddr, socket: &mut net::UdpSocket, q: Question) {
	let mut reply = Vec::<u8>::new();
	reply.extend(q.id);

	if q.domain_name.len() == 1 && q.domain_name[0] == b"my-server" {
		// Ok.
		reply.extend({
			use bitvec::prelude::*;
			
			let mut bits = BitVec::<u8, Msb0>::new();
	
			bits.extend_from_bitslice(&1u8.view_bits::<Msb0>()[8 - 1..]); // It is a response.
			bits.extend_from_bitslice(&0u8.view_bits::<Msb0>()[8 - 4..]); // Assumption: copy that request was a query.
			bits.extend_from_bitslice(&1u8.view_bits::<Msb0>()[8 - 1..]); // Assumption: the answer is authoritative.
			bits.extend_from_bitslice(&0u8.view_bits::<Msb0>()[8 - 1..]); // Assumption: the reply is not truncated.
			bits.extend_from_bitslice(&1u8.view_bits::<Msb0>()[8 - 1..]); // Assumption: copy that recursion was desired.
			bits.extend_from_bitslice(&0u8.view_bits::<Msb0>()[8 - 1..]); // Assumption: recursion is not supported.
			bits.extend_from_bitslice(&0u8.view_bits::<Msb0>()[8 - 3..]); // A reserved field.
			bits.extend_from_bitslice(&0u8.view_bits::<Msb0>()[8 - 4..]); // Assumption: the response is not an error.
	
			bits.extend_from_bitslice(&0u16.view_bits::<Msb0>()[..]); // Assumption: there are no questions.
			bits.extend_from_bitslice(&1u16.view_bits::<Msb0>()[..]); // Assumption: there is an answer.
			bits.extend_from_bitslice(&0u16.view_bits::<Msb0>()[..]); // Assumption: there are no name servers.
			bits.extend_from_bitslice(&0u16.view_bits::<Msb0>()[..]); // Assumption: there are no additional records.
	
			bits.into_vec()
		});
	
		for part in &q.domain_name {
			assert!(part.len() < 256);
			reply.push(part.len() as u8);
			reply.extend(part);
		}
		reply.push(0u8);
	
		reply.extend({
			use bitvec::prelude::*;
	
			let mut bits = BitVec::<u8, Msb0>::new();
	
			bits.extend_from_bitslice(&1u16.view_bits::<Msb0>()[..]); // It is a host address.
			bits.extend_from_bitslice(&1u16.view_bits::<Msb0>()[..]); // The host address is in the Internet.
			bits.extend_from_bitslice(&0u32.view_bits::<Msb0>()[..]); // Time to live is zero.
			bits.extend_from_bitslice(&4u16.view_bits::<Msb0>()[..]); // The IP address has four octets.
	
			// The IP address.
			bits.extend_from_bitslice(&172u8.view_bits::<Msb0>()[..]);
			bits.extend_from_bitslice(&16u8.view_bits::<Msb0>()[..]);
			bits.extend_from_bitslice(&0u8.view_bits::<Msb0>()[..]);
			bits.extend_from_bitslice(&4u8.view_bits::<Msb0>()[..]);
			
			bits.into_vec()
		});
	} else {
		// Refused.
		reply.extend({
			use bitvec::prelude::*;
			
			let mut bits = BitVec::<u8, Msb0>::new();
	
			bits.extend_from_bitslice(&1u8.view_bits::<Msb0>()[8 - 1..]); // It is a response.
			bits.extend_from_bitslice(&0u8.view_bits::<Msb0>()[8 - 4..]); // Assumption: copy that request was a query.
			bits.extend_from_bitslice(&1u8.view_bits::<Msb0>()[8 - 1..]); // Assumption: the answer is authoritative.
			bits.extend_from_bitslice(&0u8.view_bits::<Msb0>()[8 - 1..]); // Assumption: the reply is not truncated.
			bits.extend_from_bitslice(&1u8.view_bits::<Msb0>()[8 - 1..]); // Assumption: copy that recursion was desired.
			bits.extend_from_bitslice(&0u8.view_bits::<Msb0>()[8 - 1..]); // Assumption: recursion is not supported.
			bits.extend_from_bitslice(&0u8.view_bits::<Msb0>()[8 - 3..]); // A reserved field.
			bits.extend_from_bitslice(&5u8.view_bits::<Msb0>()[8 - 4..]); // REFUSED.
	
			bits.extend_from_bitslice(&0u16.view_bits::<Msb0>()[..]); // Assumption: there are no questions.
			bits.extend_from_bitslice(&0u16.view_bits::<Msb0>()[..]); // Assumption: there are no answers.
			bits.extend_from_bitslice(&0u16.view_bits::<Msb0>()[..]); // Assumption: there are no name servers.
			bits.extend_from_bitslice(&0u16.view_bits::<Msb0>()[..]); // Assumption: there are no additional records.
	
			bits.into_vec()
		});
	}

	socket.send_to(&reply, source).unwrap();
}

// Fix me: convert to proper endianness when reading multibyte values of constant size.
fn handle(socket: &mut net::UdpSocket, request: &[u8], source: SocketAddr) {
	let question = parse(&request);

	reply(source, socket, question);
}

fn main() {
	let mut socket = {
		use std::os::fd::AsRawFd;
		use nix::sys::socket::{
			socket, bind, setsockopt,
			AddressFamily, SockType, SockFlag, sockopt::ReuseAddr, SockaddrIn,
		};
		let fd = socket(
			AddressFamily::Inet, SockType::Datagram, SockFlag::empty(), None /* Does this matter? */
		).unwrap();
		setsockopt(&fd, ReuseAddr, &true).unwrap();
		bind(fd.as_raw_fd(), &SockaddrIn::new(127, 0, 0, 1, 8000)).unwrap();
		net::UdpSocket::from(fd)
	};

	loop {
		let mut request = [0u8; 2 << 16];
		let (request, from) = {
			let (length, from) = socket.recv_from(&mut request).unwrap();
			(&request[0..length], from)
		};
		handle(&mut socket, request, from);
	}
}
