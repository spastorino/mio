use {localhost, TryWrite};
use mio::*;
use mio::tcp::*;
use bytes::SliceBuf;

const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

struct TestHandler {
    server: TcpListener,
    client: TcpStream,
    state: usize,
}

impl TestHandler {
    fn new(srv: TcpListener, cli: TcpStream) -> TestHandler {
        TestHandler {
            server: srv,
            client: cli,
            state: 0,
        }
    }

    fn handle_read(&mut self, poll: &mut Poll, token: Token) {
        match token {
            SERVER => {
                trace!("handle_read; token=SERVER");
                let mut sock = self.server.accept().unwrap().0;
                sock.try_write_buf(&mut SliceBuf::wrap("foobar".as_bytes())).unwrap();
            }
            CLIENT => {
                trace!("handle_read; token=CLIENT");
                assert!(self.state == 0, "unexpected state {}", self.state);
                self.state = 1;
                poll.reregister(&self.client, CLIENT, Ready::writable(), PollOpt::level()).unwrap();
            }
            _ => panic!("unexpected token"),
        }
    }

    fn handle_write(&mut self, poll: &mut Poll, token: Token) {
        debug!("handle_write; token={:?}; state={:?}", token, self.state);

        assert!(token == CLIENT, "unexpected token {:?}", token);
        assert!(self.state == 1, "unexpected state {}", self.state);

        self.state = 2;
        poll.deregister(&self.client).unwrap();
    }
}

#[test]
pub fn test_register_deregister() {
    let _ = ::env_logger::init();

    debug!("Starting TEST_REGISTER_DEREGISTER");
    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1024);

    let addr = localhost();

    let server = TcpListener::bind(&addr).unwrap();

    info!("register server socket");
    poll.register(&server, SERVER, Ready::readable(), PollOpt::edge()).unwrap();

    let client = TcpStream::connect(&addr).unwrap();

    // Register client socket only as writable
    poll.register(&client, CLIENT, Ready::readable(), PollOpt::level()).unwrap();

    let mut handler = TestHandler::new(server, client);

    loop {
        poll.poll(&mut events, None).unwrap();

        let event = events.get(0).unwrap();

        if event.readiness().is_readable() {
            handler.handle_read(&mut poll, event.token());
        }

        if event.readiness().is_writable() {
            handler.handle_write(&mut poll, event.token());
            break;
        }
    }
}

#[test]
pub fn test_register_with_no_readable_writable_is_error() {
    let poll = Poll::new().unwrap();
    let addr = localhost();

    let sock = TcpListener::bind(&addr).unwrap();

    assert!(poll.register(&sock, Token(0), Ready::hup(), PollOpt::edge()).is_err());

    poll.register(&sock, Token(0), Ready::readable(), PollOpt::edge()).unwrap();

    assert!(poll.reregister(&sock, Token(0), Ready::hup(), PollOpt::edge()).is_err());
}
