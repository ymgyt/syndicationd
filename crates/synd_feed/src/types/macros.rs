macro_rules! impl_sqlx_encode_decode {
    ($ty:ty as String) => {
        #[cfg(feature = "sqlx")]
        impl sqlx::Type<sqlx::Sqlite> for $ty {
            fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
                <String as sqlx::Type<sqlx::Sqlite>>::type_info()
            }
        }

        #[cfg(feature = "sqlx")]
        impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for $ty {
            fn encode_by_ref(
                &self,
                buf: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
            ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
                self.to_string().encode_by_ref(buf)
            }
        }

        #[cfg(feature = "sqlx")]
        impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for $ty {
            fn decode(
                value: sqlx::sqlite::SqliteValueRef<'r>,
            ) -> Result<Self, sqlx::error::BoxDynError> {
                let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
                Self::try_from(s.as_str()).map_err(Into::into)
            }
        }
    };
}

pub(crate) use impl_sqlx_encode_decode;
