use crate::errors::NonEmptyVecError;

#[derive(Debug, Clone)]
pub struct NonEmptyVec<T>(Vec<T>);

impl<T> NonEmptyVec<T> {
    pub fn new(vec: Vec<T>) -> Result<Self, NonEmptyVecError> {
        if vec.is_empty() {
            Err(NonEmptyVecError::EmptyVector)
        } else {
            Ok(Self(vec))
        }
    }

    pub fn into_inner(self) -> Vec<T> {
        self.0
    }

    pub fn inner(&self) -> &Vec<T> {
        &self.0
    }

    pub fn first(&self) -> &T {
        unsafe { self.0.get_unchecked(0) }
    }
}
