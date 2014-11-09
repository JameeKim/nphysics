use std::any::AnyRefExt;
use std::rc::Rc;
use std::cell::RefCell;
use std::intrinsics::TypeId;
use std::num::One;
use std::collections::HashMap;
use rand::{SeedableRng, XorShiftRng, Rng};
use rsfml::graphics::RenderWindow;
use na::{Pnt3, Iso2};
use nphysics::object::RigidBody;
use ncollide::shape::Shape2;
use ncollide::shape;
use camera::Camera;
use objects::ball::Ball;
use objects::box_node::Box;
use objects::lines::Lines;

pub enum SceneNode<'a> {
    BallNode(Ball<'a>),
    BoxNode(Box<'a>),
    LinesNode(Lines)
}

impl<'a> SceneNode<'a> {
    pub fn select(&mut self) {
        match *self {
            BallNode(ref mut n) => n.select(),
            BoxNode(ref mut n) => n.select(),
            LinesNode(ref mut n) => n.select(),
        }
    }

    pub fn unselect(&mut self) {
        match *self {
            BallNode(ref mut n) => n.unselect(),
            BoxNode(ref mut n) => n.unselect(),
            LinesNode(ref mut n) => n.unselect(),
        }
    }
}

pub struct GraphicsManager<'a> {
    rand:      XorShiftRng,
    rb2sn:     HashMap<uint, Vec<SceneNode<'a>>>,
    obj2color: HashMap<uint, Pnt3<u8>>
}

impl<'a> GraphicsManager<'a> {
    pub fn new() -> GraphicsManager<'a> {
        GraphicsManager {
            rand:      SeedableRng::from_seed([0, 1, 2, 3]),
            rb2sn:     HashMap::new(),
            obj2color: HashMap::new()
        }
    }

    pub fn add(&mut self, body: Rc<RefCell<RigidBody>>) {

        let nodes = {
            let rb    = body.borrow();
            let mut nodes = Vec::new();

            self.add_geom(body.clone(), One::one(), rb.geom_ref(), &mut nodes);

            nodes
        };

        self.rb2sn.insert(body.deref() as *const RefCell<RigidBody> as uint, nodes);
    }

    fn add_geom(&mut self,
                body:  Rc<RefCell<RigidBody>>,
                delta: Iso2<f32>,
                geom:  &Shape2,
                out:   &mut Vec<SceneNode<'a>>) {
        type Pl = shape::Plane2;
        type Bl = shape::Ball2;
        type Bo = shape::Cuboid2;
        type Cy = shape::Cylinder2;
        type Co = shape::Cone2;
        type Cm = shape::Compound2;
        type Ls = shape::Mesh2;

        let id = geom.get_type_id();
        if id == TypeId::of::<Pl>(){
            self.add_plane(body, geom.downcast_ref::<Pl>().unwrap(), out)
        }
        else if id == TypeId::of::<Bl>() {
            self.add_ball(body, delta, geom.downcast_ref::<Bl>().unwrap(), out)
        }
        else if id == TypeId::of::<Bo>() {
            self.add_box(body, delta, geom.downcast_ref::<Bo>().unwrap(), out)
        }
        else if id == TypeId::of::<Cm>() {
            let c = geom.downcast_ref::<Cm>().unwrap();

            for &(t, ref s) in c.geoms().iter() {
                self.add_geom(body.clone(), delta * t, &***s, out)
            }
        }
        else if id == TypeId::of::<Ls>() {
            self.add_lines(body, delta, geom.downcast_ref::<Ls>().unwrap(), out)
        }
        else {
            panic!("Not yet implemented.")
        }

    }

    fn add_plane(&mut self,
                 _: Rc<RefCell<RigidBody>>,
                 _: &shape::Plane2,
                 _: &mut Vec<SceneNode>) {
    }

    fn add_ball(&mut self,
                body:  Rc<RefCell<RigidBody>>,
                delta: Iso2<f32>,
                geom:  &shape::Ball2,
                out:   &mut Vec<SceneNode>) {
        let color = self.color_for_object(&body);
        let margin = body.borrow().margin();
        out.push(BallNode(Ball::new(body, delta, geom.radius() + margin, color)))
    }

    fn add_lines(&mut self,
               body:  Rc<RefCell<RigidBody>>,
               delta: Iso2<f32>,
               geom:  &shape::Mesh2,
               out:   &mut Vec<SceneNode>) {

        let color = self.color_for_object(&body);

        let vs = geom.vertices().clone();
        let is = geom.indices().clone();

        out.push(LinesNode(Lines::new(body, delta, vs, is, color)))
    }


    fn add_box(&mut self,
               body:  Rc<RefCell<RigidBody>>,
               delta: Iso2<f32>,
               geom:  &shape::Cuboid2,
               out:   &mut Vec<SceneNode>) {
        let rx = geom.half_extents().x;
        let ry = geom.half_extents().y;
        let margin = body.borrow().margin();

        let color = self.color_for_object(&body);

        out.push(BoxNode(Box::new(body, delta, rx + margin, ry + margin, color)))
    }

    pub fn clear(&mut self) {
        self.rb2sn.clear();
    }

    pub fn draw(&mut self, rw: &mut RenderWindow, c: &Camera) {
        c.activate_scene(rw);

        for (_, ns) in self.rb2sn.iter_mut() {
            for n in ns.iter_mut() {
                match *n {
                    BoxNode(ref mut b)   => b.update(),
                    BallNode(ref mut b)  => b.update(),
                    LinesNode(ref mut l) => l.update(),
                }
            }
        }

        for (_, ns) in self.rb2sn.iter_mut() {
            for n in ns.iter_mut() {
                match *n {
                    BoxNode(ref b)   => b.draw(rw),
                    BallNode(ref b)  => b.draw(rw),
                    LinesNode(ref l) => l.draw(rw),
                }
            }
        }

        c.activate_ui(rw);
    }

    pub fn set_color(&mut self, body: &Rc<RefCell<RigidBody>>, color: Pnt3<u8>) {
        let key = body.deref() as *const RefCell<RigidBody> as uint;
        self.obj2color.insert(key, color);
    }

    pub fn color_for_object(&mut self, body: &Rc<RefCell<RigidBody>>) -> Pnt3<u8> {
        let key = body.deref() as *const RefCell<RigidBody> as uint;
        match self.obj2color.get(&key) {
            Some(color) => return *color,
            None => { }
        }

        let color = Pnt3::new(
            self.rand.gen_range(0u, 256) as u8,
            self.rand.gen_range(0u, 256) as u8,
            self.rand.gen_range(0u, 256) as u8);


        self.obj2color.insert(key, color);

        color
    }

    pub fn body_to_scene_node(&mut self, rb: &Rc<RefCell<RigidBody>>) -> Option<&mut Vec<SceneNode<'a>>> {
        self.rb2sn.get_mut(&(rb.deref() as *const RefCell<RigidBody> as uint))
    }
}