use gen::fabric_hello_server;

#[allow(non_snake_case)]
pub mod gen {
    tonic::include_proto!("fabrichello"); // The string specified here must match the proto package name
    tonic::include_proto!("todolist"); // The string specified here must match the proto package name
}

pub struct HelloSvcImpl {}

#[tonic::async_trait]
impl fabric_hello_server::FabricHelloService for HelloSvcImpl {
    async fn say_hello(
        &self,
        request: gen::FabricRequest,
    ) -> Result<gen::FabricResponse, tonic::Status> {
        let name = request.fabric_name;
        let mut msg_reply = String::from("Hello: ");
        msg_reply.push_str(name.as_str());
        let reply = gen::FabricResponse {
            fabric_message: msg_reply,
        };
        Ok(reply)
    }
}

pub mod todolist {
    use std::{collections::HashMap, sync::Mutex};

    use crate::gen::{AddOneResponse, DeleteOneResponse, FindRequest, FindResponse};

    #[derive(Clone)]
    struct Item {
        pub id: i32,
        pub description: String,
        pub completed: bool,
    }

    impl Item {
        fn into_proto(self) -> super::gen::Item {
            super::gen::Item {
                id: self.id,
                description: self.description,
                completed: self.completed,
            }
        }

        fn from_proto(proto: &super::gen::Item) -> Item {
            Item {
                id: proto.id,
                description: proto.description.clone(),
                completed: proto.completed,
            }
        }
    }

    #[derive(Default)]
    pub struct TodoSvcImpl {
        entries: Mutex<HashMap<i32, Item>>,
    }

    impl TodoSvcImpl {
        fn find(&self) -> Vec<Item> {
            let entries = self.entries.lock().unwrap();
            entries.values().cloned().collect()
        }

        // returns true if added. false if duplicated.
        fn add_one(&self, item: Item) -> bool {
            let mut entries = self.entries.lock().unwrap();
            if entries.contains_key(&item.id) {
                return false;
            }
            entries.insert(item.id, item);
            true
        }

        fn delete_one(&self, id: i32) -> Option<Item> {
            let mut entries = self.entries.lock().unwrap();
            if !entries.contains_key(&id) {
                return None;
            }
            // delete the entry
            entries.remove(&id)
        }
    }

    #[tonic::async_trait]
    impl super::gen::todo_server::TodoService for TodoSvcImpl {
        async fn find(
            &self,
            _request: FindRequest,
        ) -> Result<super::gen::FindResponse, tonic::Status> {
            let items: Vec<crate::gen::Item> =
                self.find().iter().map(|x| x.clone().into_proto()).collect();
            Ok(FindResponse { items })
        }

        async fn add_one(
            &self,
            request: super::gen::AddOneRequest,
        ) -> Result<super::gen::AddOneResponse, tonic::Status> {
            if request.payload.is_none() {
                return Err(tonic::Status::invalid_argument("empty payload"));
            }
            let item = request.payload.unwrap();
            let ok = self.add_one(Item::from_proto(&item));
            if !ok {
                return Err(tonic::Status::already_exists("entry already exist"));
            }
            let resp = AddOneResponse {
                payload: Some(item),
            };
            Ok(resp)
        }

        async fn delete_one(
            &self,
            request: super::gen::DeleteOneRequest,
        ) -> Result<super::gen::DeleteOneResponse, tonic::Status> {
            let item = self.delete_one(request.id);
            match item {
                Some(i) => Ok(DeleteOneResponse {
                    payload: Some(i.into_proto()),
                }),
                None => Err(tonic::Status::not_found("id not found")),
            }
        }
    }
}

#[cfg(test)]
mod generator_test {
    use fabric_rpc_rs::server::Server;
    use windows::core::HSTRING;

    use crate::{
        gen::{
            fabric_hello_client::FabricHelloClient, fabric_hello_server, todo_client::TodoClient,
            todo_server::TodoServiceRouter, AddOneRequest, DeleteOneRequest, FabricRequest,
            FindRequest, Item,
        },
        todolist::TodoSvcImpl,
        HelloSvcImpl,
    };

    #[tokio::test]
    async fn helloworldtest() {
        // open server
        let (stoptx, stoprx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            // make server run
            let hello_svc = HelloSvcImpl {};

            let mut svr = Server::default();
            svr.add_service(fabric_hello_server::FabricHelloServiceRouter::new(
                hello_svc,
            ));
            svr.serve_with_shutdown(12347, async {
                stoprx.await.ok();
                println!("Graceful shutdown complete")
            })
            .await;
        });

        let connectionaddress = HSTRING::from("localhost:12347+/");

        let helloclient = FabricHelloClient::connect(connectionaddress).await.unwrap();

        // send request
        {
            let request = FabricRequest {
                fabric_name: String::from("myname"),
            };
            let resp = helloclient.say_hello(10000, request).await.unwrap();

            assert_eq!("Hello: myname", resp.fabric_message);
        }
        {
            let request = FabricRequest {
                fabric_name: String::from("myname"),
            };
            let resp = helloclient.say_hello(10000, request).await.unwrap();

            assert_eq!("Hello: myname", resp.fabric_message);
        }

        // stop server
        stoptx.send(()).unwrap();
    }

    #[tokio::test]
    async fn todotest() {
        // open server
        let (stoptx, stoprx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            // make server run
            let todo_svc = TodoSvcImpl::default();

            let mut svr = Server::default();
            svr.add_service(TodoServiceRouter::new(todo_svc));
            svr.serve_with_shutdown(12348, async move { stoprx.await.unwrap() })
                .await;
        });

        let connectionaddress = HSTRING::from("localhost:12348+/");

        let todoclient = TodoClient::connect(connectionaddress).await.unwrap();

        // send request
        {
            let item1 = Item {
                id: 1,
                description: "first".to_string(),
                completed: false,
            };
            let request = AddOneRequest {
                payload: Some(item1),
            };
            let resp = todoclient.add_one(1000, request).await.unwrap();
            assert_eq!(1, resp.payload.unwrap().id);
        }

        {
            let item = Item {
                id: 2,
                description: "second".to_string(),
                completed: false,
            };
            let request = AddOneRequest {
                payload: Some(item),
            };
            let resp = todoclient.add_one(1000, request).await.unwrap();
            assert_eq!(2, resp.payload.unwrap().id);
        }

        {
            let request = FindRequest {};
            let resp = todoclient.find(1000, request).await.unwrap();
            assert_eq!(2, resp.items.len());
        }

        {
            let request = DeleteOneRequest { id: 1 };
            let resp = todoclient.delete_one(1000, request).await.unwrap();
            assert_eq!(1, resp.payload.unwrap().id);
        }

        {
            let request = FindRequest {};
            let resp = todoclient.find(1000, request).await.unwrap();
            assert_eq!(1, resp.items.len());
        }

        // stop server
        stoptx.send(()).unwrap();
    }
}
