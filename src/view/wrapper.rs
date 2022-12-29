use std::any::Any;
use crate::event::EventResult;
use crate::id::Id;
use crate::View;
use crate::view::Cx;
use crate::view::sequence::ViewSequence;
use crate::widget::{ChangeFlags, Pod};

trait Wrapper<T, A = ()> {
    type State: Send;
    type V;

    fn state(&self) -> Self::State;

    fn gen(&self, state: &mut Self::State) -> V;

    /// Propagate an event.
    ///
    /// Handle an event, propagating to children if needed. Here, `id_path` is a slice
    /// of ids beginning at a child of this view.
    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A>;
}

impl<T, A, W: Wrapper<T, A>> View<T, A> for W
    where Self::V: View<T, A>
{
    type State = (<W as Wrapper<T, A>>::State, Self::V, Self::V::State);
    type Element = ();

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let mut wrapper_state = self.state();

        let view = self.gen(&mut wrapper_state);
        let (id, view_state, element) = view.build(cx);

        (id, (wrapper_state, view, view_state), element)
    }

    fn rebuild(&self, cx: &mut Cx, prev: &Self, id: &mut Id, state: &mut Self::State, element: &mut Self::Element) -> ChangeFlags {
        let current = self.gen(&mut state.0);

        current.rebuild(cx, &state.1, id, &mut state.2, element)
    }

    fn event(&self, id_path: &[Id], state: &mut Self::State, event: Box<dyn Any>, app_state: &mut T) -> EventResult<A> {
        self.event(id_path, &mut state.0, event, app_state)
            .or(|event|{
                state.1.event(id_path, &mut state.2, event, app_state)
            })
    }
}

//TODO: impl ViewSequence for Wrapper...