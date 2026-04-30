pub mod hello;
pub mod big;

#[cfg(test)]
mod roundtrip {
    use super::big::demo::{Drawer, DrawerProxy, DrawerServant, Point, Shape, Color};
    use super::hello::test_app::{Hello, HelloProxy, HelloServant};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tars_core::Context;
    use tars_protocol::protocol::{RequestPacket, ResponsePacket};
    use tars_transport::transport::ServerProtocolHandler;

    // ---- HelloImpl ----
    struct HelloImpl;
    #[async_trait]
    impl Hello for HelloImpl {
        async fn hello(&self, _ctx: &Context, no: i32, name: String) -> tars_core::Result<String> {
            Ok(format!("hello#{} {}", no, name))
        }
    }

    /// Build a request packet via the proxy-encoding helpers, run it through the
    /// servant dispatcher, then decode the response.
    #[tokio::test]
    async fn hello_round_trip() {
        let servant = HelloServant::new(HelloImpl);

        // Manually craft a request the same way HelloProxy::hello would.
        use tars_core::codec::{Buffer, TarsEncode};
        let mut buf = Buffer::new();
        42i32.encode(&mut buf, 1).unwrap();
        "world".to_string().encode(&mut buf, 2).unwrap();

        let mut req = RequestPacket::new();
        req.s_func_name = "hello".to_string();
        req.s_buffer = buf.to_bytes();
        let pkg = req.encode().unwrap();

        let mut ctx = Context::new();
        let resp_bytes = servant.invoke(&mut ctx, &pkg).await;
        let resp = ResponsePacket::decode(&resp_bytes).unwrap();
        assert!(resp.is_success(), "i_ret = {} desc = {}", resp.i_ret, resp.s_result_desc);

        use tars_core::codec::{Reader, TarsDecode};
        let mut reader = Reader::new(&resp.s_buffer);
        let ret: String = String::decode(&mut reader, 0, true).unwrap();
        assert_eq!(ret, "hello#42 world");
    }

    // ---- DrawerImpl: exercises struct, vector, map, enum, out-param, void ----
    struct DrawerImpl;
    #[async_trait]
    impl Drawer for DrawerImpl {
        async fn draw(&self, _ctx: &Context, input: Shape, repeats: i32) -> tars_core::Result<Shape> {
            let mut out = input.clone();
            // duplicate the points `repeats` times
            for _ in 0..repeats.saturating_sub(1) {
                out.points.extend(input.points.iter().cloned());
            }
            Ok(out)
        }

        async fn count(&self, _ctx: &Context, values: Vec<i32>) -> tars_core::Result<(i32, Vec<i32>)> {
            let doubled: Vec<i32> = values.iter().map(|v| v * 2).collect();
            Ok((values.len() as i32, doubled))
        }

        async fn noop(&self, _ctx: &Context) -> tars_core::Result<()> {
            Ok(())
        }
    }

    fn sample_shape() -> Shape {
        let mut tags = HashMap::new();
        tags.insert("kind".to_string(), 1i32);
        tags.insert("layer".to_string(), 7i32);
        Shape {
            name: "tri".to_string(),
            points: vec![
                Point { x: 0, y: 0, label: "origin".to_string() },
                Point { x: 1, y: 2, label: "".to_string() },
                Point { x: 3, y: 4, label: "tip".to_string() },
            ],
            color: Color::GREEN,
            tags,
        }
    }

    /// Drive draw(...) through the dispatcher.
    #[tokio::test]
    async fn drawer_draw_round_trip() {
        let servant = DrawerServant::new(DrawerImpl);

        // Encode args the way DrawerProxy::draw does.
        use tars_core::codec::{Buffer, TarsEncode};
        let shape = sample_shape();
        let mut buf = Buffer::new();
        shape.encode(&mut buf, 1).unwrap();
        2i32.encode(&mut buf, 2).unwrap();

        let mut req = RequestPacket::new();
        req.s_func_name = "draw".to_string();
        req.s_buffer = buf.to_bytes();
        let pkg = req.encode().unwrap();

        let mut ctx = Context::new();
        let resp_bytes = servant.invoke(&mut ctx, &pkg).await;
        let resp = ResponsePacket::decode(&resp_bytes).unwrap();
        assert!(resp.is_success(), "i_ret = {} desc = {}", resp.i_ret, resp.s_result_desc);

        use tars_core::codec::{Reader, TarsDecode};
        let mut reader = Reader::new(&resp.s_buffer);
        let ret: Shape = Shape::decode(&mut reader, 0, true).unwrap();
        assert_eq!(ret.points.len(), 6); // 3 points × 2 repeats
        assert_eq!(ret.color, Color::GREEN);
        assert_eq!(ret.tags.get("kind"), Some(&1));
        assert_eq!(ret.points[0].label, "origin");
    }

    /// Drive count(...) — exercises Vec<i32> in/out and tuple return.
    #[tokio::test]
    async fn drawer_count_round_trip() {
        let servant = DrawerServant::new(DrawerImpl);

        use tars_core::codec::{Buffer, TarsEncode, TarsType};
        let values = vec![1i32, 2, 3, 4];
        let mut buf = Buffer::new();
        // Inline list encode (matches DrawerProxy::count)
        buf.write_head(TarsType::List, 1).unwrap();
        buf.write_int32(values.len() as i32, 0).unwrap();
        for v in &values {
            v.encode(&mut buf, 0).unwrap();
        }

        let mut req = RequestPacket::new();
        req.s_func_name = "count".to_string();
        req.s_buffer = buf.to_bytes();
        let pkg = req.encode().unwrap();

        let mut ctx = Context::new();
        let resp_bytes = servant.invoke(&mut ctx, &pkg).await;
        let resp = ResponsePacket::decode(&resp_bytes).unwrap();
        assert!(resp.is_success(), "i_ret = {} desc = {}", resp.i_ret, resp.s_result_desc);

        use tars_core::codec::{Reader, TarsDecode};
        let mut reader = Reader::new(&resp.s_buffer);
        let count: i32 = i32::decode(&mut reader, 0, true).unwrap();
        let len = reader.read_list_begin(2, true).unwrap();
        let mut doubled: Vec<i32> = Vec::with_capacity(len as usize);
        for _ in 0..len {
            doubled.push(i32::decode(&mut reader, 0, true).unwrap());
        }
        assert_eq!(count, 4);
        assert_eq!(doubled, vec![2, 4, 6, 8]);
    }

    /// Compile-time check: HelloProxy is constructible from an Arc<ServantProxy>.
    #[test]
    fn proxy_compiles() {
        fn _take_servant_proxy(p: Arc<tars_rpc::servant::ServantProxy>) -> HelloProxy {
            HelloProxy::new(p)
        }
        fn _take_drawer(p: Arc<tars_rpc::servant::ServantProxy>) -> DrawerProxy {
            DrawerProxy::new(p)
        }
        let _ = _take_servant_proxy;
        let _ = _take_drawer;
    }
}
